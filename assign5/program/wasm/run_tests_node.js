// Headless test runner for assign5 mystery.wat.
//
// The course ships a browser harness (src/index.js + src/tests.js) that runs
// under webpack with jQuery/chai. This runner exercises the exact same five
// cases from src/tests.js, but headlessly: it compiles mystery.wat with
// `wat2wasm` (wabt) and instantiates it with Node's built-in WebAssembly.
//
//   node run_tests_node.js
//
const { execFileSync } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");

const watPath = path.join(__dirname, "src", "mystery.wat");
const wasmPath = path.join(os.tmpdir(), "mystery_assign5.wasm");

// Compile the WebAssembly text to a binary module.
execFileSync("wat2wasm", [watPath, "-o", wasmPath]);
const bytes = fs.readFileSync(wasmPath);

// Same (input, expected) pairs as src/tests.js.
const cases = [
  [1, 1],
  [3, 8],
  [12, 10],
  [100, 26],
  [1000, 112],
];

(async () => {
  const { instance } = await WebAssembly.instantiate(bytes, {});
  const mystery = instance.exports.mystery;
  let pass = 0;
  let fail = 0;
  for (const [n, r] of cases) {
    const got = mystery(n);
    const ok = got === r;
    console.log(
      `${ok ? "PASS" : "FAIL"}: mystery(${n}) = ${got} (expected ${r})`
    );
    ok ? pass++ : fail++;
  }
  console.log(`\n${pass}/${cases.length} tests passed`);
  process.exit(fail ? 1 : 0);
})();
