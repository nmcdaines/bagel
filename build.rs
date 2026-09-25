// Rebuild when migrations change so `sqlx::migrate!()` re-embeds them.
fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
