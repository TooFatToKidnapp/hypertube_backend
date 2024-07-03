use std::env;

pub fn check_for_necessary_env() {
    env::var("FRONTEND_URL").expect("FRONTEND_URL must be set");
    env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    env::var("JWT_SECRET").expect("JWT_SECRET must be set");
}
