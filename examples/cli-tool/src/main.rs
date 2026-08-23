#![forbid(unsafe_code)]

mod config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let loaded = config::load_from_process()?;
    println!("profile: {:?}", loaded.profile);
    println!("jobs: {}", loaded.jobs);
    println!("minify: {}", loaded.minify);
    println!("fail on warning: {}", loaded.fail_on_warning);
    println!("include paths: {}", loaded.include_paths.join(", "));
    println!(
        "registry token configured: {}",
        loaded.registry_token.is_some()
    );
    Ok(())
}
