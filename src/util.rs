use std::env;

pub fn check_for_necessary_env() -> Result<(), std::io::Error> {
    env::var("FRONTEND_URL").map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("FRONTEND_URL {}.", e)))?;
    env::var("DATABASE_URL").map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("DATABASE_URL {}.", e)))?;
    env::var("JWT_SECRET").map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("JWT_SECRET {}.", e)))?;
    env::var("CLIENT_UID_42").map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("CLIENT_UID_42 {}.", e)))?;
    env::var("CLIENT_SECRET_42").map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("CLIENT_SECRET_42 {}.", e)))?;
    env::var("BACKEND_URL").map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("BACKEND_URL {}.", e)))?;
    env::var("GOOGLE_CLIENT_ID").map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("GOOGLE_CLIENT_ID {}.", e)))?;
    env::var("GOOGLE_CLIENT_SECRET").map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("GOOGLE_CLIENT_SECRET {}.", e)))?;
    env::var("GOOGLE_CLIENT_SCOPE").map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("GOOGLE_CLIENT_SCOPE {}.", e)))?;
    env::var("FAILURE_REDIRECT_URI").map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("FAILURE_REDIRECT_URI {}.", e)))?;
    env::var("CLIENT_ID_GITHUB").map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("CLIENT_ID_GITHUB {}.", e)))?;
    env::var("CLIENT_SECRET_GITHUB").map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("CLIENT_SECRET_GITHUB {}.", e)))?;
    env::var("CLIENT_SCOPE_GITHUB").map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("CLIENT_SCOPE_GITHUB {}.", e)))?;
    env::var("EMAIL_SENDER_USERNAME").map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("EMAIL_SENDER_USERNAME {}.", e)))?;
    env::var("EMAIL_SENDER_PASSWORD").map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("EMAIL_SENDER_PASSWORD {}.", e)))?;
    env::var("S3_BUCKET_SECRET_KEY").map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("S3_BUCKET_SECRET_KEY {}.", e)))?;
    env::var("S3_BUCKET_ACCESS_KEY").map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("S3_BUCKET_ACCESS_KEY {}.", e)))?;
    env::var("S3_BUCKET_NAME").map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("S3_BUCKET_NAME {}.", e)))?;
    env::var("S3_PROVIDER_URL").map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("S3_PROVIDER_URL {}.", e)))?;
    env::var("S3_BUCKET_URL").map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("S3_BUCKET_URL {}.", e)))?;
    env::var("S3_REGION").map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("S3_REGION {}.", e)))?;
    env::var("MOVIE_DB_AUTH_TOKEN").map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("MOVIE_DB_AUTH_TOKEN {}.", e)))?;
    Ok(())
}
