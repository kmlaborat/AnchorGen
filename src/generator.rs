pub fn get_generator<'a>(
    config: &'a crate::config::Config,
    name: &str,
) -> Result<&'a crate::config::GeneratorSpec, String> {
    config
        .generators
        .get(name)
        .ok_or_else(|| format!("Generator '{}' not found", name))
}
