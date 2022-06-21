// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use super::{app, icon::create_icns_file};
use crate::{
  bundle::{
    common::{copy_dir, CommandExt},
    Bundle,
  },
  PackageType::MacOsBundle,
  Settings,
};

use anyhow::Context;
use log::info;
use walkdir::WalkDir;

use std::{
  env,
  fs::{self, write},
  path::PathBuf,
  process::{Command, Stdio},
};

/// Bundles the project.
/// Returns a vector of PathBuf that shows where the DMG was created.
pub fn bundle_project(settings: &Settings, bundles: &[Bundle]) -> crate::Result<Vec<PathBuf>> {
  // generate the .app bundle if needed
  if bundles
    .iter()
    .filter(|bundle| bundle.package_type == MacOsBundle)
    .count()
    == 0
  {
    app::bundle_project(settings)?;
  }

  // get the target path
  let output_path = settings.project_out_directory().join("bundle/dmg");
  let package_base_name = format!(
    "{}_{}_{}",
    settings.main_binary_name(),
    settings.version_string(),
    match settings.binary_arch() {
      "x86_64" => "x64",
      other => other,
    }
  );
  let dmg_name = format!("{}.dmg", &package_base_name);
  let dmg_path = output_path.join(&dmg_name);

  let product_name = settings.main_binary_name();
  let bundle_file_name = format!("{}.app", product_name);
  let bundle_dir = settings.project_out_directory().join("bundle/macos");

  info!(action = "Bundling"; "{} ({})", dmg_name, dmg_path.display());

  if output_path.exists() {
    fs::remove_dir_all(&output_path)
      .with_context(|| format!("Failed to remove old {}", dmg_name))?;
  }

  // Holds all the files that will be turned into the DMG
  let temp_dir = output_path.join("temp");
  fs::create_dir_all(&temp_dir)
    .with_context(|| format!("Failed to create temporary directory at {:?}", temp_dir))?;

  let support_dir = temp_dir.join("support");
  fs::create_dir_all(&support_dir)
    .with_context(|| format!("Failed to create support directory at {:?}", support_dir))?;

  // Copy the .app file
  copy_dir(
    &bundle_dir.join(&bundle_file_name),
    &temp_dir.join(&bundle_file_name),
  )
  .context("Failed to copy .app to temp folder to create DMG")?;

  // create paths for script
  // let bundle_script_path = output_path.join("bundle_dmg.sh");

  // write the scripts
  // write(
  //   &bundle_script_path,
  //   include_str!("templates/dmg/bundle_dmg"),
  // )?;
  // write(
  //   support_directory_path.join("template.applescript"),
  //   include_str!("templates/dmg/template.applescript"),
  // )?;

  write(
    support_dir.join("eula-resources-template.xml"),
    include_str!("templates/dmg/eula-resources-template.xml"),
  )?;

  // // chmod script for execution
  // Command::new("chmod")
  //   .arg("777")
  //   .arg(&bundle_script_path)
  //   .current_dir(&output_path)
  //   .stdout(Stdio::piped())
  //   .stderr(Stdio::piped())
  //   .output()
  //   .expect("Failed to chmod script");

  // let mut args = vec![
  //   "--volname",
  //   product_name,
  //   "--icon",
  //   product_name,
  //   "180",
  //   "170",
  //   "--app-drop-link",
  //   "480",
  //   "170",
  //   "--window-size",
  //   "660",
  //   "400",
  //   "--hide-extension",
  //   &bundle_file_name,
  // ];

  let icns_icon_path =
    create_icns_file(&temp_dir, settings)?.map(|path| path.to_string_lossy().to_string());
  if let Some(icon) = &icns_icon_path {
    // Currently not copying it over
    fs::copy(icon, temp_dir.join(".VolumeIcon.icns"))
      .context("Failed to create the DMG volume icon")?;
  }

  #[allow(unused_assignments)]
  let mut license_path_ref = "".to_string();
  if let Some(license_path) = &settings.macos().license {
    // args.push("--eula");
    // license_path_ref = env::current_dir()?
    //   .join(license_path)
    //   .to_string_lossy()
    //   .to_string();
    // args.push(&license_path_ref);
  }

  // Issue #592 - Building macOS dmg files on CI
  // https://github.com/tauri-apps/tauri/issues/592
  if let Some(value) = env::var_os("CI") {
    if value == "true" {
      // args.push("--skip-jenkins");
    }
  }

  println!("bundle_dir {:?}", bundle_dir);
  println!("output_path {:?}", output_path);
  println!("dmg_name {:?}", dmg_name);
  println!("product_name {:?}", product_name);
  println!("bundle_file_name {:?}", bundle_file_name);

  // Make a new directory and place license_path_ref, icns_icon_path

  // Place .VolumeIcon.icns in directory
  // fs::copy(bundle_file_name.clone(), bundle_dir.clone())
  //   .context("Copying icon")?;

  Command::new("hdiutil")
    .current_dir(bundle_dir.clone())
    .arg("create")
    .arg(dmg_name.as_str())
    .arg("-volname")
    .arg(product_name)
    .arg("-fs")
    .arg("HFS+")
    // https://ss64.com/osx/hdiutil.html
    // .arg("-fsargs")
    // .arg("\"-c c=64,a=16,e=16\"")
    .arg("-srcfolder")
    .arg(temp_dir.clone())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .output()
    .context("Error creating DMG for macOS")?;

  // execute the bundle script
  // Command::new(&bundle_script_path)
  //   .current_dir(bundle_dir.clone())
  //   .args(args)
  //   .args(vec![dmg_name.as_str(), bundle_file_name.as_str()])
  //   .output_ok()
  //   .context("error running bundle_dmg.sh")?;

  fs::rename(bundle_dir.join(dmg_name), dmg_path.clone())?;

  // Sign DMG if needed
  if let Some(identity) = &settings.macos().signing_identity {
    super::sign::sign(dmg_path.clone(), identity, settings, false)?;
  }

  Ok(vec![dmg_path])
}
