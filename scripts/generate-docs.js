const fs = require("fs");
const cp = require("child_process");
const path = require("path");

const outDir = path.join(process.cwd(), "docs-out");
const workspace = path.join(outDir, "workspace");
const manifest = path.join(workspace, "Cargo.toml");
const docSource = path.join(workspace, "target", "doc");
const docDest = path.join(process.cwd(), "docs", "static", "rustdoc");

function copyAll(source, dest) {
    if (fs.lstatSync(source).isDirectory()) {
        fs.mkdirSync(dest);
        for (const child of fs.readdirSync(source)) {
            copyAll(path.join(source, child), path.join(dest, child));
        }
    } else {
        fs.copyFileSync(source, dest);
    }
}

if (fs.existsSync(outDir)) {
    fs.rmSync(outDir, { recursive: true });
}

if (fs.existsSync(docDest)) {
    fs.rmSync(docDest, { recursive: true });
}

cp.execFileSync("cargo", ["run", "--release", "--", "examples/json.js", "--out-dir", outDir]);
cp.execFileSync("cargo", ["new", "--lib", "--vcs", "none", "--name", "parser", "workspace"], { cwd: outDir });

fs.rmSync(path.join(workspace, "src", "lib.rs"));

fs.writeFileSync(manifest, `
[package]
name = "parser"
version = "0.1.0"
edition = "2021"

[lib]
path = "../parser.rs"
`);

cp.execFileSync("cargo", ["doc"], { cwd: workspace });

copyAll(docSource, docDest);
fs.rmSync(outDir, { recursive: true });
