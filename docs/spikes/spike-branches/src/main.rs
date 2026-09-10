use bower_spike_branches::{fixtures, lock_text, plan, replay};

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "/tmp/bower-spike-failers".into());
    let steps = fixtures::rank_saga();
    let p = plan(&steps).expect("the saga plans");
    println!("=== bower.lock (proposed shape) ===\n{}", lock_text(&p));
    let report = replay::run(&p, std::path::Path::new(&out)).expect("replay");
    println!("=== replayed {} commits into {out} ===", report.commits);
    for (r, sha) in &report.refs {
        println!("{sha}  {r}");
    }
    println!("\n=== git log --graph ===\n{}", replay::graph(std::path::Path::new(&out)));

    println!("=== plan-time errors, one fixture each ===");
    for (name, f) in [
        ("conflict_unresolved", fixtures::conflict_unresolved()),
        ("step_after_merge", fixtures::step_after_merge()),
        ("merge_unknown", fixtures::merge_unknown()),
        ("merge_twice", fixtures::merge_twice()),
        ("from_not_on_main", fixtures::from_not_on_main()),
    ] {
        match plan(&f) {
            Ok(_) => println!("{name}: (no error — unexpected)"),
            Err(es) => for e in es { println!("{name}: {e}"); },
        }
    }
}
