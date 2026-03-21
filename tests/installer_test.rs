use plugin_store::installer::skill::SkillInstaller;
use tempfile::TempDir;

#[tokio::test]
async fn test_skill_install_creates_directory_and_file() {
    let tmp = TempDir::new().unwrap();
    let skill_dir = tmp.path().join("test-plugin");
    let content = "# Test Skill\nThis is a test.";

    SkillInstaller::write_skill(&skill_dir, content).unwrap();

    let skill_path = skill_dir.join("SKILL.md");
    assert!(skill_path.exists());
    assert_eq!(std::fs::read_to_string(&skill_path).unwrap(), content);
}
