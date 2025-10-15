#[napi(object)]
#[derive(Debug, Clone)]
pub struct ProjectPaths {
  pub root: String,
  pub cache: String,
  pub artifacts: String,
  pub sources: String,
  pub tests: String,
  pub scripts: String,
  pub libraries: Vec<String>,
}
