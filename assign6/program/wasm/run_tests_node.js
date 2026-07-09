// Headless test runner for the assign6 OCaml SLang -> WebAssembly compiler.
//
// The course ships a browser harness (src/index.js + src/tests.js) that runs
// under webpack. This runner reproduces src/tests.js exactly, but headlessly:
//
//   * it compiles the generated basic.wat / concat.wat / funcall.wat with
//     `wat2wasm` (wabt). Those files are produced by `make && ./main.native`
//     in ../ocaml, i.e. by the student's Translate/Slang code.
//   * it instantiates the *provided* allocator (wasm-alloc/wasm_alloc_bg.wasm)
//     and the *provided* memcpy.wasm, wiring them to each SLang module exactly
//     as index.js does ("wasm-alloc" + "memcpy" imports, memory = memcpy.memory).
//   * it runs main(), decodes the returned (ptr,len) string out of memcpy's
//     memory, and -- like tests.js -- asserts the output and that no allocation
//     was leaked (count() must be 1 afterwards).
//
// wabt 1.0.34 only accepts the modern instruction spelling, whereas the course's
// wasm.ml emits the pre-2019 names (get_local/set_local/get_global/set_global).
// We translate those tokens before compiling; the generated .wat is left as the
// compiler wrote it.
//
//   node run_tests_node.js
//
const { execFileSync } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");
const assert = require("assert");

const SRC = path.join(__dirname, "src");
const ALLOC = path.join(__dirname, "wasm-alloc", "wasm_alloc_bg.wasm");
const MEMCPY = path.join(SRC, "memcpy.wasm");

function compileWat(name) {
  let text = fs.readFileSync(path.join(SRC, name + ".wat"), "utf8");
  text = text
    .replace(/\bget_local\b/g, "local.get")
    .replace(/\bset_local\b/g, "local.set")
    .replace(/\bget_global\b/g, "global.get")
    .replace(/\bset_global\b/g, "global.set");
  const tmpWat = path.join(os.tmpdir(), `a6_${name}.wat`);
  const tmpWasm = path.join(os.tmpdir(), `a6_${name}.wasm`);
  fs.writeFileSync(tmpWat, text);
  execFileSync("wat2wasm", [tmpWat, "-o", tmpWasm]);
  return fs.readFileSync(tmpWasm);
}

// Decode a SLang string: one 32-bit word per character, in memcpy's memory.
function toString(mem32, ptr, len) {
  let s = "";
  for (let i = 0; i < len; i++) s += String.fromCharCode(mem32[ptr + i]);
  return s;
}

(async () => {
  // Provided allocator. Its only imports are wasm-bindgen logging/panic shims
  // that are never called on valid input.
  const allocStub = () => 0;
  const allocImports = {
    "./wasm_alloc.js": {
      __wbg_log_0c692f8f7e856d13: allocStub,
      __wbg_new_59cb74e423758ede: allocStub,
      __wbg_stack_558ba5917b466edd: allocStub,
      __wbg_error_4bb6c2a97407129a: allocStub,
      __wbindgen_object_drop_ref: allocStub,
    },
  };
  const alloc = (await WebAssembly.instantiate(fs.readFileSync(ALLOC), allocImports))
    .instance.exports;

  // Provided memcpy module (exports memcpy + the shared data memory).
  const memcpy = (await WebAssembly.instantiate(fs.readFileSync(MEMCPY), {}))
    .instance.exports;
  const mem32 = new Uint32Array(memcpy.memory.buffer);

  const slangImports = {
    "wasm-alloc": { alloc: alloc.alloc, dealloc: alloc.dealloc },
    memcpy: { memcpy: memcpy.memcpy, memory: memcpy.memory },
  };

  const cases = [
    { name: "basic", expect: "basic" },
    { name: "concat", expect: "hello world" },
    { name: "funcall", expect: "hello world" },
  ];

  let pass = 0;
  for (const { name, expect } of cases) {
    const bytes = compileWat(name);
    const mod = (await WebAssembly.instantiate(bytes, slangImports)).instance.exports;
    alloc.alloc_init();
    const ptr = mod.main();
    const got = toString(mem32, ptr, expect.length);
    let ok = true;
    let detail = "";
    try {
      assert.strictEqual(got, expect, "output mismatch");
      const remaining = alloc.count();
      assert.ok(remaining <= 1, `${remaining - 1} allocation(s) leaked`);
    } catch (e) {
      ok = false;
      detail = " -- " + e.message;
    }
    console.log(`${ok ? "PASS" : "FAIL"}: test_${name} -> "${got}"${detail}`);
    if (ok) pass++;
  }
  console.log(`\n${pass}/${cases.length} tests passed`);
  process.exit(pass === cases.length ? 0 : 1);
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
