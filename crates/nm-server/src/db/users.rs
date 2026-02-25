use nm_common::models::{CreateUser, User};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn get_by_email(pool: &PgPool, email: &str) -> anyhow::Result<Option<User>> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1 AND is_active = true")
        .bind(email)
        .fetch_optional(pool)
        .await?;
    Ok(user)
}

pub async fn get_by_id(pool: &PgPool, id: Uuid) -> anyhow::Result<Option<User>> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(user)
}

pub async fn create(pool: &PgPool, input: &CreateUser, password_hash: &str) -> anyhow::Result<User> {
    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (email, password_hash, display_name, role) VALUES ($1, $2, $3, $4) RETURNING *"
    )
        .bind(&input.email)
        .bind(password_hash)
        .bind(&input.display_name)
        .bind(&input.role)
        .fetch_one(pool)
        .await?;
    Ok(user)
}

pub async fn list(pool: &PgPool) -> anyhow::Result<Vec<User>> {
    let users = sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY created_at")
        .fetch_all(pool)
        .await?;
    Ok(users)
}

pub async fn delete(pool: &PgPool, id: Uuid) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_last_login(pool: &PgPool, id: Uuid) -> anyhow::Result<()> {
    sqlx::query("UPDATE users SET last_login_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
