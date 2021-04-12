use lazy_static::lazy_static;
use serde_json::{json, Value};
use crate::Scoop;

lazy_static! {
  static ref KNOWN_BUCKETS: Value = json!({
    "main": "https://github.com/ScoopInstaller/Main",
    "extras": "https://github.com/lukesampson/scoop-extras",
    "versions": "https://github.com/ScoopInstaller/Versions",
    "nightlies": "https://github.com/ScoopInstaller/Nightlies",
    "nirsoft": "https://github.com/kodybrown/scoop-nirsoft",
    "php": "https://github.com/ScoopInstaller/PHP",
    "nerd-fonts": "https://github.com/matthewjberger/scoop-nerd-fonts",
    "nonportable": "https://github.com/TheRandomLabs/scoop-nonportable",
    "java": "https://github.com/ScoopInstaller/Java",
    "games": "https://github.com/Calinou/scoop-games",
    "jetbrains": "https://github.com/Ash258/Scoop-JetBrains"
  });
}

impl Scoop {
  pub fn get_known_buckets() {
    let buckets = KNOWN_BUCKETS.as_object().unwrap().keys();
    for b in buckets {
      println!("{}", b);
    }
  }

  pub fn get_known_bucket_url(bucket_name: &str) -> &'static str {
    KNOWN_BUCKETS[bucket_name].as_str().unwrap()
  }

  pub fn get_added_buckets(&self) -> Vec<String> {
    let buckets = std::fs::read_dir(&self.buckets_dir).unwrap();
    let mut ret: Vec<String> = Vec::new();

    for b in buckets {
      ret.push(b.unwrap().file_name().to_str().unwrap().to_owned());
    }

    ret
  }

  pub fn is_known_bucket(bucket_name: &str) -> bool {
    KNOWN_BUCKETS.as_object().unwrap().contains_key(bucket_name)
  }

  pub fn buckets(&self) {
    let buckets = self.get_added_buckets();
    for b in buckets {
      println!("{}", b);
    }
  }

  pub fn is_added_bucket(&self, bucket_name: &str) -> bool {
    let buckets = self.get_added_buckets();
    buckets.contains(&bucket_name.to_string())
  }
}
