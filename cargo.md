# Ok voici ce que tu dois faire dans l'ordre #

cargo check
cargo clippy
cargo clippy -- -D warnings
cargo fmt -- --check
cargo fmt
cargo check --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
cargo test --all-features
cargo test --doc --all-features
cargo test

ensuite tu vas faire cargo test et si un ou plusieurs tests passent pas alors corrige l'ai pour que ça passe le module qui se fait tester bienssur sinon c'est le test que tu corrige

Attention : Tu ne dois JAMAIS supprimé, un code ou un module. tu le corrige. c'est c'est un mot que tu dois corriger tu le corrige pas tous le bloque. Dans les code je veux 0 tests, 0 autorisation, 0 texte provisoire, code mort, incoherences et surtout je veux 0 allow quand tu corrige si tu vois tu retires.
