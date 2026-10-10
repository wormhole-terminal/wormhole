#!/usr/bin/env bash
# A small Rust project: four commits over a week, one modified file, one new file staged,
# one untracked file.
#   tools/visual/fixtures/repo-basic.sh <directory to create>
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"

write README.md <<'EOF'
# orbit

Plots a course between two points.
EOF
write Cargo.toml <<'EOF'
[package]
name = "orbit"
version = "0.1.0"
edition = "2024"
EOF
write src/main.rs <<'EOF'
use orbit::Course;

fn main() {
    let course = Course::between((0.0, 0.0), (3.0, 4.0));
    println!("distance: {}", course.distance());
}
EOF
write src/lib.rs <<'EOF'
/// A straight line from one point to another.
pub struct Course {
    from: (f64, f64),
    to: (f64, f64),
}

impl Course {
    pub fn between(from: (f64, f64), to: (f64, f64)) -> Self {
        Self { from, to }
    }

    pub fn distance(&self) -> f64 {
        let (dx, dy) = (self.to.0 - self.from.0, self.to.1 - self.from.1);
        (dx * dx + dy * dy).sqrt()
    }
}
EOF
commit 7 "Start orbit: a course between two points"

write src/fuel.rs <<'EOF'
/// Fuel needed for a distance, at a fixed burn rate.
pub fn needed(distance: f64, burn_per_unit: f64) -> f64 {
    distance * burn_per_unit
}
EOF
printf '\npub mod fuel;\n' >> "$repo/src/lib.rs"
commit 4 "Add fuel estimates"

write README.md <<'EOF'
# orbit

Plots a course between two points and estimates the fuel it takes.

    cargo run
EOF
commit 2 "Describe fuel in the README"

write tests/course.rs <<'EOF'
use orbit::Course;

#[test]
fn a_three_four_five_triangle() {
    assert_eq!(Course::between((0.0, 0.0), (3.0, 4.0)).distance(), 5.0);
}
EOF
commit 1 "Test the distance"

# The working tree: a change to the course, a staged new file, and notes nobody added yet.
write src/lib.rs <<'EOF'
/// A straight line from one point to another, with a name for the log.
pub struct Course {
    name: String,
    from: (f64, f64),
    to: (f64, f64),
}

impl Course {
    pub fn between(from: (f64, f64), to: (f64, f64)) -> Self {
        Self { name: "unnamed".into(), from, to }
    }

    pub fn named(mut self, name: &str) -> Self {
        self.name = name.into();
        self
    }

    pub fn distance(&self) -> f64 {
        let (dx, dy) = (self.to.0 - self.from.0, self.to.1 - self.from.1);
        dx.hypot(dy)
    }
}

pub mod fuel;
EOF
write src/log.rs <<'EOF'
/// One line per course plotted.
pub fn entry(name: &str, distance: f64) -> String {
    format!("{name}: {distance:.1}")
}
EOF
git -C "$repo" add src/log.rs
write NOTES.md <<'EOF'
- check the burn rate against the manual
EOF
