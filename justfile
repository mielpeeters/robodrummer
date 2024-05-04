alias doc := build-doc
alias b := build


build-doc:
    cargo doc --no-deps
    echo "<meta http-equiv=\"refresh\" content=\"0; url=robodrummer\">" > target/doc/index.html
    cp -r target/doc/* ./docs/

build:
    cargo build --release

train:
    @read -p "Enter the name of the training data: " data; \
    cargo run --release train -d $data 

generate: 
    RUST_LOG=debug cargo run --release generate-data -v 6 euclidean -k 3 -n 8
