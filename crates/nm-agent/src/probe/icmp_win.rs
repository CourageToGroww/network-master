//! Windows ICMP probe using IcmpSendEcho API.
//! This does NOT require admin/elevated privileges, unlike raw sockets.

use std::net::{IpAddr, Ipv4Addr};

use super::ProbeResult;

// Windows API FFI declarations
#[cfg(windows)]
mod ffi {
    use std::ffi::c_void;

    pub type HANDLE = *mut c_void;
    pub type DWORD = u32;
    pub type ULONG = u32;
    pub type USHORT = u16;
    pub type UCHAR = u8;
    pub type BOOL = i32;
    pub type IPAddr = u32; // IPv4 address in network byte order

    pub const INVALID_HANDLE_VALUE: HANDLE = -1isize as HANDLE;

    // IP_STATUS codes
    pub const IP_SUCCESS: ULONG = 0;
    pub const IP_TTL_EXPIRED_TRANSIT: ULONG = 11013;
    pub const IP_DEST_NET_UNREACHABLE: ULONG = 11002;
    pub const IP_DEST_HOST_UNREACHABLE: ULONG = 11003;
    pub const IP_DEST_PROT_UNREACHABLE: ULONG = 11004;
    pub const IP_DEST_PORT_UNREACHABLE: ULONG = 11005;

    #[repr(C)]
    pub struct IP_OPTION_INFORMATION {
        pub ttl: UCHAR,
        pub tos: UCHAR,
        pub flags: UCHAR,
        pub options_size: UCHAR,
        pub options_data: *mut UCHAR,
    }

    #[repr(C)]
    pub struct ICMP_ECHO_REPLY {
        pub address: IPAddr,
        pub status: ULONG,
        pub round_trip_time: ULONG,
        pub data_size: USHORT,
        pub reserved: USHORT,
        pub data: *mut c_void,
        pub options: IP_OPTION_INFORMATION,
    }

    #[link(name = "iphlpapi")]
    extern "system" {
        pub fn IcmpCreateFile() -> HANDLE;
        pub fn IcmpCloseHandle(handle: HANDLE) -> BOOL;
        pub fn IcmpSendEcho(
            icmp_handle: HANDLE,
            destination_address: IPAddr,
            request_data: *const c_void,
            request_size: USHORT,
            request_options: *mut IP_OPTION_INFORMATION,
            reply_buffer: *mut c_void,
            reply_size: DWORD,
            timeout: DWORD,
        ) -> DWORD;
    }

    #[link(name = "kernel32")]
    extern "system" {
        pub fn GetLastError() -> DWORD;
    }
}

#[cfg(windows)]
pub async fn send_icmp_probe(
    dest: IpAddr,
    ttl: u8,
    packet_size: u16,
    timeout_ms: u64,
) -> ProbeResult {
    let IpAddr::V4(dest_v4) = dest else {
        // IPv6 not supported via IcmpSendEcho (would need Icmp6SendEcho2)
        return ProbeResult {
            hop_number: ttl,
            responding_ip: None,
            rtt_us: None,
            timed_out: true,
            ttl_received: None,
        };
    };

    // Run blocking Windows API call in spawn_blocking
    tokio::task::spawn_blocking(move || {
        send_icmp_probe_win(dest_v4, ttl, packet_size, timeout_ms)
    })
    .await
    .unwrap_or(ProbeResult {
        hop_number: ttl,
        responding_ip: None,
        rtt_us: None,
        timed_out: true,
        ttl_received: None,
    })
}

#[cfg(windows)]
fn send_icmp_probe_win(
    dest: Ipv4Addr,
    ttl: u8,
    packet_size: u16,
    timeout_ms: u64,
) -> ProbeResult {
    use ffi::*;
    use std::ffi::c_void;

    let fail = ProbeResult {
        hop_number: ttl,
        responding_ip: None,
        rtt_us: None,
        timed_out: true,
        ttl_received: None,
    };

    unsafe {
        let handle = IcmpCreateFile();
        if handle == INVALID_HANDLE_VALUE || handle.is_null() {
            let err = GetLastError();
            tracing::warn!(error_code = err, "IcmpCreateFile failed");
            return fail;
        }

        // IP address in network byte order (big-endian u32)
        let octets = dest.octets();
        let dest_addr: IPAddr =
            (octets[0] as u32)
            | ((octets[1] as u32) << 8)
            | ((octets[2] as u32) << 16)
            | ((octets[3] as u32) << 24);

        // Request data (payload)
        let payload_size = (packet_size as usize).saturating_sub(8).max(1);
        let request_data = vec![0x41u8; payload_size]; // fill with 'A'

        // Set TTL via IP_OPTION_INFORMATION
        let mut options = IP_OPTION_INFORMATION {
            ttl,
            tos: 0,
            flags: 0,
            options_size: 0,
            options_data: std::ptr::null_mut(),
        };

        // Reply buffer must be large enough for ICMP_ECHO_REPLY + data
        let reply_size = std::mem::size_of::<ICMP_ECHO_REPLY>() + payload_size + 8;
        let mut reply_buffer = vec![0u8; reply_size];

        let num_replies = IcmpSendEcho(
            handle,
            dest_addr,
            request_data.as_ptr() as *const c_void,
            payload_size as USHORT,
            &mut options,
            reply_buffer.as_mut_ptr() as *mut c_void,
            reply_size as DWORD,
            timeout_ms as DWORD,
        );

        if num_replies == 0 {
            let err = GetLastError();
            // 11010 = IP_REQ_TIMED_OUT (expected for some hops)
            if err != 11010 {
                tracing::debug!(
                    ttl = ttl,
                    dest = %dest,
                    error_code = err,
                    reply_buf_size = reply_size,
                    "IcmpSendEcho failed"
                );
            }
        }

        // IcmpSendEcho returns 0 for error statuses like TTL_EXPIRED,
        // but still fills the reply buffer. Check the buffer regardless.
        let reply = &*(reply_buffer.as_ptr() as *const ICMP_ECHO_REPLY);

        let result = if num_replies > 0 || reply.status != 0 {
            // Convert reply IP address from network byte order to Ipv4Addr
            let reply_ip = Ipv4Addr::new(
                (reply.address & 0xFF) as u8,
                ((reply.address >> 8) & 0xFF) as u8,
                ((reply.address >> 16) & 0xFF) as u8,
                ((reply.address >> 24) & 0xFF) as u8,
            );

            match reply.status {
                IP_SUCCESS => {
                    // Reached the destination
                    ProbeResult {
                        hop_number: ttl,
                        responding_ip: Some(IpAddr::V4(reply_ip)),
                        rtt_us: Some((reply.round_trip_time as u32) * 1000), // ms to us
                        timed_out: false,
                        ttl_received: Some(reply.options.ttl),
                    }
                }
                IP_TTL_EXPIRED_TRANSIT => {
                    // Intermediate router responded
                    ProbeResult {
                        hop_number: ttl,
                        responding_ip: Some(IpAddr::V4(reply_ip)),
                        rtt_us: Some((reply.round_trip_time as u32) * 1000), // ms to us
                        timed_out: false,
                        ttl_received: None,
                    }
                }
                IP_DEST_NET_UNREACHABLE
                | IP_DEST_HOST_UNREACHABLE
                | IP_DEST_PROT_UNREACHABLE
                | IP_DEST_PORT_UNREACHABLE => {
                    // Destination unreachable - still got a response
                    ProbeResult {
                        hop_number: ttl,
                        responding_ip: Some(IpAddr::V4(reply_ip)),
                        rtt_us: Some((reply.round_trip_time as u32) * 1000),
                        timed_out: false,
                        ttl_received: None,
                    }
                }
                11010 => {
                    // IP_REQ_TIMED_OUT - no response
                    fail
                }
                _ => {
                    tracing::debug!(
                        status = reply.status,
                        ttl = ttl,
                        reply_ip = %reply_ip,
                        "ICMP reply with status"
                    );
                    fail
                }
            }
        } else {
            fail
        };

        IcmpCloseHandle(handle);
        result
    }
}

#[cfg(not(windows))]
pub async fn send_icmp_probe(
    dest: IpAddr,
    ttl: u8,
    packet_size: u16,
    timeout_ms: u64,
) -> ProbeResult {
    // Fall back to raw socket implementation on non-Windows
    super::icmp::send_icmp_probe(dest, ttl, packet_size, timeout_ms).await
}
