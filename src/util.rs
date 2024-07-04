use std::env;

pub fn check_for_necessary_env() {
    env::var("FRONTEND_URL").expect("FRONTEND_URL env must be set");
    env::var("DATABASE_URL").expect("DATABASE_URL env must be set");
    env::var("JWT_SECRET").expect("JWT_SECRET env must be set");
    env::var("CLIENT_UID_42").expect("CLIENT_UID_42 env must be set");
    env::var("CLIENT_SECRET_42").expect("CLIENT_SECRET_42 env must be set");
    env::var("BACKEND_URL").expect("BACKEND_URL env must be set");
    env::var("GOOGLE_CLIENT_ID").expect("GOOGLE_CLIENT_ID env must be set");
    env::var("GOOGLE_CLIENT_SECRET").expect("GOOGLE_CLIENT_SECRET env must be set");
    env::var("GOOGLE_CLIENT_SCOPE").expect("GOOGLE_CLIENT_SCOPE env must be set");
    env::var("FAILURE_REDIRECT_URI").expect("FAILURE_REDIRECT_URI env must be set");
    env::var("GITHUB_CLIENT_ID").expect("GITHUB_CLIENT_ID env must be set");
    env::var("GITHUB_CLIENT_SECRET").expect("GITHUB_CLIENT_SECRET env must be set");
    env::var("GITHUB_CLIENT_SCOPE").expect("GITHUB_CLIENT_SCOPE env must be set");
}
