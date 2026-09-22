use std::{collections::BTreeMap, fs};
use tempfile::tempdir;
use zenpi::config::{self, AuthFile, ConfigOverrides, ConfigPaths};

fn explicit_workspace(user: &str) -> (tempfile::TempDir, ConfigPaths, std::path::PathBuf) {
    let root = tempdir().unwrap();
    let paths = ConfigPaths::for_home(root.path());
    fs::create_dir_all(&paths.root).unwrap();
    fs::write(&paths.config, user).unwrap();
    let workspace = root.path().join("project");
    fs::create_dir_all(workspace.join(".zenpi")).unwrap();
    (root, paths, workspace)
}

#[test]
fn project_cannot_rebind_any_trusted_explicit_profile_field_even_to_same_value() {
    let user = "default_profile='approved'\n[profiles.approved]\nprovider='deepseek'\nwire_api='responses'\nauth_method='api_key'\nauth_ref='approved-key'\nmodel='deepseek-flash'\n";
    let (_root, paths, workspace) = explicit_workspace(user);
    for field in [
        "provider='deepseek'",
        "backend='openai'",
        "wire_api='chat'",
        "base_url='https://api.deepseek.com'",
        "auth_ref='other-key'",
        "auth_method='legacy_api_key'",
        "auth_method='api_key'",
        "auth_header='bearer'",
        "auth_env='PROJECT_KEY'",
        "model_routes=[]",
        "requires_openai_auth=false",
        "supports_websockets=true",
    ] {
        fs::write(
            workspace.join(".zenpi/config.toml"),
            format!("[profiles.approved]\n{field}\n"),
        )
        .unwrap();
        let error = config::load_workspace_config(&paths, Some(&workspace)).unwrap_err();
        assert!(
            error.to_string().contains("project config"),
            "{field}: {error}"
        );
        assert_eq!(fs::read_to_string(&paths.config).unwrap(), user);
    }
}

#[test]
fn project_cannot_create_explicit_authentication_or_upgrade_a_legacy_profile() {
    let (_root, paths, workspace) = explicit_workspace("[profiles.legacy]\nmodel='old'\n");
    for overlay in [
        "provider='openai-codex'\nauth_method='oauth'\nauth_ref='cred'",
        "[profiles.new]\nprovider='openai-codex'\nauth_method='oauth'\nauth_ref='cred'",
        "[profiles.legacy]\nprovider='openai'\nwire_api='responses'\nauth_method='api_key'\nauth_ref='cred'",
        "[profiles.new]\nprovider='local'\nwire_api='chat'\nbase_url='http://localhost:8080/v1'\nauth_method='none'",
    ] {
        fs::write(workspace.join(".zenpi/config.toml"), overlay).unwrap();
        assert!(
            config::load_workspace_config(&paths, Some(&workspace)).is_err(),
            "{overlay}"
        );
    }
}

#[test]
fn project_may_change_only_nonsecret_parameters_of_the_user_selected_explicit_profile() {
    let user = "default_profile='approved'\n[profiles.approved]\nprovider='deepseek'\nwire_api='responses'\nauth_method='api_key'\nauth_ref='approved-key'\nmodel='user-model'\nbase_url='https://api.deepseek.com'\n";
    let (_root, paths, workspace) = explicit_workspace(user);
    fs::write(
        workspace.join(".zenpi/config.toml"),
        "[profiles.approved]\nmodel='project-model'\nmax_retries=2\ntimeout_seconds=60\n",
    )
    .unwrap();
    let merged = config::load_workspace_config(&paths, Some(&workspace)).unwrap();
    let environment = BTreeMap::from([
        ("ZENPI_PROVIDER".into(), "ambient-provider".into()),
        ("ZENPI_BASE_URL".into(), "https://wrong.test/v1".into()),
        ("ZENPI_API_KEY".into(), "synthetic-unrelated-key".into()),
    ]);
    let r = config::resolve(
        &ConfigOverrides::default(),
        &merged,
        &AuthFile::default(),
        &environment,
    )
    .unwrap();
    assert_eq!(r.model.as_deref(), Some("project-model"));
    assert_eq!(r.provider.as_deref(), Some("deepseek"));
    assert_eq!(r.auth_ref.as_deref(), Some("approved-key"));
    assert_eq!(r.base_url.as_deref(), Some("https://api.deepseek.com"));
    assert!(r.api_key.is_none());
    assert_eq!(r.max_retries, Some(2));
    assert_eq!(r.timeout_seconds, Some(60));
    assert_eq!(fs::read_to_string(&paths.config).unwrap(), user);
}

#[test]
fn project_cannot_select_explicit_accounts_or_downgrade_user_selected_authentication() {
    let explicit = "provider='deepseek'\nwire_api='responses'\nauth_method='api_key'\nauth_ref='approved-key'\nmodel='user-model'\n";
    let cases = [
        (
            format!("[profiles.approved]\n{explicit}"),
            "default_profile='approved'",
        ),
        (
            format!("default_profile='approved'\n[profiles.approved]\n{explicit}"),
            "default_profile='approved'",
        ),
        (
            format!(
                "default_profile='approved'\n[profiles.approved]\n{explicit}[profiles.legacy]\nmodel='old'\n"
            ),
            "default_profile='legacy'",
        ),
        (
            format!("default_profile='approved'\n[profiles.approved]\n{explicit}"),
            "default_profile='new'\n[profiles.new]\nmodel='new'",
        ),
        (
            explicit.to_owned(),
            "default_profile='new'\n[profiles.new]\nmodel='new'",
        ),
    ];
    for (user, overlay) in cases {
        let (_root, paths, workspace) = explicit_workspace(&user);
        fs::write(workspace.join(".zenpi/config.toml"), overlay).unwrap();
        let error = config::load_workspace_config(&paths, Some(&workspace)).unwrap_err();
        assert!(
            error.to_string().contains(
                "project config cannot select or change explicit authentication profiles"
            ),
            "{overlay}: {error}"
        );
        assert_eq!(fs::read_to_string(&paths.config).unwrap(), user);
    }
}

#[test]
fn project_cannot_override_flat_explicit_connection_including_empty_route_array() {
    let user = "provider='deepseek'\nwire_api='responses'\nauth_method='api_key'\nauth_ref='approved-key'\nmodel='user-model'\n";
    let (_root, paths, workspace) = explicit_workspace(user);
    for overlay in [
        "provider='other'",
        "base_url='https://wrong.test'",
        "model_routes=[]",
        "auth_method='legacy_api_key'",
        "auth_ref='other-key'",
    ] {
        fs::write(workspace.join(".zenpi/config.toml"), overlay).unwrap();
        assert!(
            config::load_workspace_config(&paths, Some(&workspace)).is_err(),
            "{overlay}"
        );
    }
    fs::write(
        workspace.join(".zenpi/config.toml"),
        "model='project-model'\n",
    )
    .unwrap();
    assert_eq!(
        config::load_workspace_config(&paths, Some(&workspace))
            .unwrap()
            .model
            .as_deref(),
        Some("project-model")
    );
}

#[test]
fn project_profile_selection_and_field_overlay_preserve_user_profiles_and_precedence() {
    let root = tempdir().unwrap();
    let paths = ConfigPaths::for_home(root.path());
    fs::create_dir_all(&paths.root).unwrap();
    let user = "default_profile='primary'\n[profiles.primary]\nmodel='user-model'\nbase_url='http://localhost:1234/v1'\nwire_api='chat'\n[profiles.spare]\nmodel='spare-model'\n";
    fs::write(&paths.config, user).unwrap();
    let workspace = root.path().join("project");
    fs::create_dir_all(workspace.join(".zenpi")).unwrap();
    let project_path = workspace.join(".zenpi/config.toml");
    fs::write(&project_path, "default_profile='spare'\n").unwrap();
    let selected = config::load_workspace_config(&paths, Some(&workspace)).unwrap();
    assert_eq!(selected.default_profile.as_deref(), Some("spare"));
    assert_eq!(selected.profiles.len(), 2);
    fs::write(
        &project_path,
        "[profiles.primary]\nmodel='project-model'\n[profiles.third]\nmodel='third-model'\n",
    )
    .unwrap();
    let merged = config::load_workspace_config(&paths, Some(&workspace)).unwrap();
    assert_eq!(merged.profiles.len(), 3);
    assert_eq!(merged.default_profile.as_deref(), Some("primary"));
    assert_eq!(
        merged.profiles["primary"].model.as_deref(),
        Some("project-model")
    );
    assert_eq!(
        merged.profiles["primary"].base_url.as_deref(),
        Some("http://localhost:1234/v1")
    );
    assert_eq!(merged.profiles["primary"].wire_api.as_deref(), Some("chat"));
    let env = BTreeMap::from([("ZENPI_MODEL".into(), "environment-model".into())]);
    let resolved = config::resolve(
        &ConfigOverrides::default(),
        &merged,
        &AuthFile::default(),
        &env,
    )
    .unwrap();
    assert_eq!(resolved.model.as_deref(), Some("environment-model"));
    let resolved = config::resolve(
        &ConfigOverrides {
            model: Some("cli-model".into()),
            ..Default::default()
        },
        &merged,
        &AuthFile::default(),
        &env,
    )
    .unwrap();
    assert_eq!(resolved.model.as_deref(), Some("cli-model"));
    assert_eq!(fs::read_to_string(&paths.config).unwrap(), user);
}

#[test]
fn project_config_rejects_invalid_merged_selection_unknown_fields_and_oversize() {
    let root = tempdir().unwrap();
    let paths = ConfigPaths::for_home(root.path());
    let workspace = root.path().join("project");
    fs::create_dir_all(workspace.join(".zenpi")).unwrap();
    let path = workspace.join(".zenpi/config.toml");
    for invalid in [
        "default_profile='missing'",
        "api_key='never-accepted'",
        "[profiles.bad]\nunknown_field=true",
        "[profiles.bad]\nmodel=''",
        "[profiles.'bad name']\nmodel='x'",
    ] {
        fs::write(&path, invalid).unwrap();
        assert!(
            config::load_workspace_config(&paths, Some(&workspace)).is_err(),
            "{invalid}"
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), invalid);
    }
    fs::write(&path, " ".repeat(262145)).unwrap();
    assert!(
        config::load_workspace_config(&paths, Some(&workspace))
            .unwrap_err()
            .to_string()
            .contains("256 KiB")
    );
    #[cfg(unix)]
    {
        fs::remove_file(&path).unwrap();
        let target = root.path().join("outside.toml");
        fs::write(&target, "model='outside'").unwrap();
        std::os::unix::fs::symlink(target, &path).unwrap();
        assert!(config::load_workspace_config(&paths, Some(&workspace)).is_err());
    }
}
