static mut GLOBALVARS: Option<VolatilityVariables> = None;

pub fn get_global_vars() -> &'static mut VolatilityVariables {
  unsafe {
    if GLOBALVARS.is_none() {
      GLOBALVARS = Some(VolatilityVariables::default());
    }
    GLOBALVARS.as_mut().unwrap()
  }
}

// ゴーストのグローバル変数のうち、揮発性(起動毎にリセットされる)のもの
pub struct VolatilityVariables {
  pub plugin_name: String,

  pub plugin_uuid: String,

  pub is_update_checked: bool,
}

impl Default for VolatilityVariables {
  fn default() -> Self {
    Self {
      plugin_name: "Ukaing".to_string(),
      plugin_uuid: "51c4eccf-406d-4da2-8aa1-fd1213a2945e".to_string(),
      is_update_checked: false,
    }
  }
}
