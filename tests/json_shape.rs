#[test]
fn history_json_roundtrip_shape() {
    let sample = serde_json::json!({
        "current_game": null,
        "history": [{
            "finished_at": "2026-09-20 13:00",
            "holes": 9,
            "results": [["Alice", 12], ["Bob", 15]],
            "winners": ["Alice"]
        }]
    });
    let dir = std::path::Path::new("data");
    let _ = std::fs::create_dir_all(dir);
    let path = dir.join("play_nine_test.json");
    std::fs::write(&path, serde_json::to_string_pretty(&sample).unwrap()).unwrap();
    let raw = std::fs::read_to_string(&path).unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(v["history"][0]["winners"][0], "Alice");
    let _ = std::fs::remove_file(path);
}
