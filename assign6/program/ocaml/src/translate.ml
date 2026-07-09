open Core

(* "s" is the scratch pointer used when building a string literal; the "cc_*"
   locals are scratch registers used by Concat (see [translate_expr]). None of
   these can collide with a SLang variable, whose locals are [x] and [x ^ "len"]. *)
let runtime_locals = ["s"; "cc_p1"; "cc_l1"; "cc_p2"; "cc_l2"; "cc_np"]

let rec translate_expr (e : Slang.expr) : Wasm.instr list =
  let open Wasm in
  match e with
  | Slang.String s ->
    let n : int = String.length s in
    let stores : instr list =
      String.to_list s
      |> List.mapi ~f:(fun (i : int) (c : char) ->
        [GetLocal "s"; Const i; Binop `Add;
         Const (Char.to_int c);
         Store])
      |> List.concat
    in
    [Const n; Call "alloc"; SetLocal "s"]
    @ stores @
    [GetLocal "s"; Const n]

  | Slang.Concat (e1, e2) ->
    (* Each operand leaves (ptr, len) on the stack. Concatenation:
         1. stash (p1, l1) and (p2, l2) in scratch locals,
         2. alloc a fresh buffer of l1 + l2 words,
         3. memcpy operand 1 to [np ..], operand 2 to [np + l1 ..]
            (the provided memcpy has argument order (src, dst, len)),
         4. free (dealloc) the two operand buffers -- they have been moved and
            must not leak,
         5. leave (np, l1 + l2) on the stack. *)
    translate_expr e1 @ [SetLocal "cc_l1"; SetLocal "cc_p1"]
    @ translate_expr e2 @ [SetLocal "cc_l2"; SetLocal "cc_p2"]
    @ [GetLocal "cc_l1"; GetLocal "cc_l2"; Binop `Add; Call "alloc"; SetLocal "cc_np"]
    @ [GetLocal "cc_p1"; GetLocal "cc_np"; GetLocal "cc_l1"; Call "memcpy"]
    @ [GetLocal "cc_p2";
       GetLocal "cc_np"; GetLocal "cc_l1"; Binop `Add;
       GetLocal "cc_l2"; Call "memcpy"]
    @ [GetLocal "cc_p1"; Call "dealloc"; GetLocal "cc_p2"; Call "dealloc"]
    @ [GetLocal "cc_np"; GetLocal "cc_l1"; GetLocal "cc_l2"; Binop `Add]

  | Slang.Call (x, es) ->
    (List.concat (List.map ~f:translate_expr es))
    @ [Call x; GetGlobal "length"]

  | Slang.Var x ->
    [GetLocal x; GetLocal (x ^ "len")]

let translate_stmt
      ((gen, locals) : (Wasm.instr list) * (string list))
      (stmt : Slang.stmt)
  : (Wasm.instr list) * (string list)
  =
  let open Slang in
  match stmt with
  | Assign (x, e) ->
    (gen @ (translate_expr e) @ [SetLocal (x ^ "len"); SetLocal x],
     [x; x ^ "len"] @ locals)
  | Return e ->
    (gen @ (translate_expr e) @ [SetGlobal "length"; Wasm.Return], locals)

let translate_func (f : Slang.func) : Wasm.func =
  let (body, locals) =
    List.fold ~init:([], runtime_locals) ~f:translate_stmt f.body
  in
  {name = f.name;
   params = List.map f.params ~f:(fun x -> [x; x ^ "len"]) |> List.concat;
   body; locals}

let translate (p : Slang.prog) : Wasm.module_ =
  List.map ~f:translate_func p
