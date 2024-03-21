#!/bin/bash
# Generate documentation for this crate

# make the documentation at `target/doc` 
cargo doc --no-deps

# add a redirect to a root-based index.html file
echo "<meta http-equiv=\"refresh\" content=\"0; url=neuroner\">" > target/doc/index.html

# copy the documentation to the `docs` folder
cp -r target/doc ./docs
