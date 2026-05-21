use awsem_core::db::DbConn;

pub struct UserPool {
    pub id: String,
    pub name: String,
    pub arn: String,
}

pub struct CognitoUser {
    pub id: String,
    pub pool_id: String,
    pub username: String,
    pub password_hash: String,
    pub email: Option<String>,
    pub attributes_json: String,
    pub status: String,
}

pub fn create_user_pool(conn: &DbConn, id: &str, name: &str, arn: &str) -> Result<(), String> {
    let c = conn.lock().map_err(|e| e.to_string())?;
    c.execute(
        "INSERT INTO cognito_user_pools (id, name, arn, config_json) VALUES (?1, ?2, ?3, '{}')",
        rusqlite::params![id, name, arn],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_user_pool(conn: &DbConn, pool_id: &str) -> Result<UserPool, String> {
    let c = conn.lock().map_err(|e| e.to_string())?;
    c.query_row(
        "SELECT id, name, arn FROM cognito_user_pools WHERE id = ?1",
        rusqlite::params![pool_id],
        |row| Ok(UserPool {
            id: row.get(0)?,
            name: row.get(1)?,
            arn: row.get(2)?,
        }),
    ).map_err(|e| e.to_string())
}

#[allow(clippy::too_many_arguments)]
pub fn create_user(
    conn: &DbConn,
    id: &str,
    pool_id: &str,
    username: &str,
    password_hash: &str,
    email: Option<&str>,
    attributes_json: &str,
    status: &str,
) -> Result<(), String> {
    let c = conn.lock().map_err(|e| e.to_string())?;
    c.execute(
        "INSERT INTO cognito_users (id, pool_id, username, password_hash, email, status, attributes_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![id, pool_id, username, password_hash, email, status, attributes_json],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn confirm_user(conn: &DbConn, username: &str, pool_id: &str) -> Result<(), String> {
    let c = conn.lock().map_err(|e| e.to_string())?;
    c.execute(
        "UPDATE cognito_users SET status = 'CONFIRMED' WHERE username = ?1 AND pool_id = ?2",
        rusqlite::params![username, pool_id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_user_by_username(conn: &DbConn, pool_id: &str, username: &str) -> Result<CognitoUser, String> {
    let c = conn.lock().map_err(|e| e.to_string())?;
    c.query_row(
        "SELECT id, pool_id, username, password_hash, email, attributes_json, status FROM cognito_users WHERE pool_id = ?1 AND username = ?2",
        rusqlite::params![pool_id, username],
        |row| Ok(CognitoUser {
            id: row.get(0)?,
            pool_id: row.get(1)?,
            username: row.get(2)?,
            password_hash: row.get(3)?,
            email: row.get(4)?,
            attributes_json: row.get(5)?,
            status: row.get(6)?,
        }),
    ).map_err(|e| e.to_string())
}
