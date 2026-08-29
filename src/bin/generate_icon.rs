use std::fs;
use std::process::Command;

fn main() {
    println!("=== Zenvi Icon & Asset Generator ===");

    let svg_path = "assets/zenvi-icon.svg";
    let svg_data = match fs::read(svg_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Could not read {}: {:?}", svg_path, e);
            return;
        }
    };

    let opt = usvg::Options::default();
    let tree = match usvg::Tree::from_data(&svg_data, &opt) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to parse SVG: {:?}", e);
            return;
        }
    };

    let assets_dir = std::path::Path::new("assets");
    let iconset_dir = assets_dir.join("AppIcon.iconset");
    fs::create_dir_all(&iconset_dir).unwrap();

    // Copy SVG to assets/icon.svg
    fs::write(assets_dir.join("icon.svg"), &svg_data).unwrap();

    let sizes: &[(u32, &str)] = &[
        (16, "icon_16x16.png"),
        (32, "icon_16x16@2x.png"),
        (32, "icon_32x32.png"),
        (64, "icon_32x32@2x.png"),
        (128, "icon_128x128.png"),
        (256, "icon_128x128@2x.png"),
        (256, "icon_256x256.png"),
        (512, "icon_256x256@2x.png"),
        (512, "icon_512x512.png"),
        (1024, "icon_512x512@2x.png"),
        (1024, "icon_1024x1024.png"),
    ];

    for &(size, filename) in sizes {
        let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size).unwrap();
        let scale = size as f32 / 1024.0;
        let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
        resvg::render(&tree, transform, &mut pixmap.as_mut());

        let out_path = if filename.starts_with("icon_1024") {
            assets_dir.join(filename)
        } else {
            iconset_dir.join(filename)
        };

        pixmap.save_png(&out_path).unwrap();
        println!("Generated: {}", out_path.display());
    }

    // Compile into macOS AppIcon.icns
    let icns_path = assets_dir.join("AppIcon.icns");
    let status = Command::new("iconutil")
        .args(&[
            "-c",
            "icns",
            iconset_dir.to_str().unwrap(),
            "-o",
            icns_path.to_str().unwrap(),
        ])
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("Successfully generated macOS AppIcon: {}", icns_path.display());
        }
        Ok(s) => eprintln!("iconutil failed with exit code: {:?}", s),
        Err(e) => eprintln!("Failed to execute iconutil: {:?}", e),
    }
}
