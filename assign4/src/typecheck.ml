open Lam_flags
open Core
open Result.Monad_infix
open Ast

exception Unimplemented

let aequiv = Ast_util.Type.aequiv

let rec typecheck_expr (ctx : Type.t String.Map.t) (e : Expr.t)
  : (Type.t, string) Result.t =
  let err fmt = Printf.ksprintf (fun s -> Error s) fmt in
  match e with
  | Expr.Num _ -> Ok Type.Num

  | Expr.Binop {left; right; _} ->
    typecheck_expr ctx left >>= fun tau_left ->
    typecheck_expr ctx right >>= fun tau_right ->
    (match (tau_left, tau_right) with
     | (Type.Num, Type.Num) -> Ok Type.Num
     | _ -> Error (
       Printf.sprintf
         "Binary operands have incompatible types: (%s : %s) and (%s : %s)"
         (Expr.to_string left) (Type.to_string tau_left)
         (Expr.to_string right) (Type.to_string tau_right)))

  | Expr.True | Expr.False -> Ok Type.Bool

  | Expr.If {cond; then_; else_} ->
    typecheck_expr ctx cond >>= fun tau_cond ->
    (match tau_cond with
     | Type.Bool ->
       typecheck_expr ctx then_ >>= fun tau_then ->
       typecheck_expr ctx else_ >>= fun tau_else ->
       if aequiv tau_then tau_else then Ok tau_then
       else err "if branches have different types: %s and %s"
           (Type.to_string tau_then) (Type.to_string tau_else)
     | _ -> err "if condition is not a bool: %s" (Type.to_string tau_cond))

  | Expr.Relop {left; right; _} ->
    typecheck_expr ctx left >>= fun tau_left ->
    typecheck_expr ctx right >>= fun tau_right ->
    (match (tau_left, tau_right) with
     | (Type.Num, Type.Num) -> Ok Type.Bool
     | _ -> err "relational operands must be num: %s and %s"
         (Type.to_string tau_left) (Type.to_string tau_right))

  | Expr.And {left; right} | Expr.Or {left; right} ->
    typecheck_expr ctx left >>= fun tau_left ->
    typecheck_expr ctx right >>= fun tau_right ->
    (match (tau_left, tau_right) with
     | (Type.Bool, Type.Bool) -> Ok Type.Bool
     | _ -> err "boolean operands must be bool: %s and %s"
         (Type.to_string tau_left) (Type.to_string tau_right))

  | Expr.Var x ->
    (match Map.find ctx x with
     | Some tau -> Ok tau
     | None -> err "Unbound variable %s" x)

  | Expr.Lam {x; tau; e} ->
    let ctx' = Map.set ctx ~key:x ~data:tau in
    typecheck_expr ctx' e >>= fun tau_ret ->
    Ok (Type.Fn {arg = tau; ret = tau_ret})

  | Expr.App {lam; arg} ->
    typecheck_expr ctx lam >>= fun tau_lam ->
    typecheck_expr ctx arg >>= fun tau_arg ->
    (match tau_lam with
     | Type.Fn {arg = tau_param; ret} ->
       if aequiv tau_param tau_arg then Ok ret
       else err "Argument type %s does not match parameter type %s"
           (Type.to_string tau_arg) (Type.to_string tau_param)
     | _ -> err "Cannot apply non-function of type %s" (Type.to_string tau_lam))

  | Expr.Unit -> Ok Type.Unit

  | Expr.Pair {left; right} ->
    typecheck_expr ctx left >>= fun tau_left ->
    typecheck_expr ctx right >>= fun tau_right ->
    Ok (Type.Product {left = tau_left; right = tau_right})

  | Expr.Project {e; d} ->
    typecheck_expr ctx e >>= fun tau ->
    (match tau with
     | Type.Product {left; right} ->
       Ok (match d with Expr.Left -> left | Expr.Right -> right)
     | _ -> err "Cannot project from non-product of type %s"
         (Type.to_string tau))

  | Expr.Inject {e; d; tau} ->
    typecheck_expr ctx e >>= fun tau_e ->
    (match tau with
     | Type.Sum {left; right} ->
       let expected = match d with Expr.Left -> left | Expr.Right -> right in
       if aequiv tau_e expected then Ok tau
       else err "Injected value type %s does not match sum component %s"
           (Type.to_string tau_e) (Type.to_string expected)
     | _ -> err "Injection annotation is not a sum type: %s"
         (Type.to_string tau))

  | Expr.Case {e; xleft; eleft; xright; eright} ->
    typecheck_expr ctx e >>= fun tau_e ->
    (match tau_e with
     | Type.Sum {left; right} ->
       let ctx_l = Map.set ctx ~key:xleft ~data:left in
       let ctx_r = Map.set ctx ~key:xright ~data:right in
       typecheck_expr ctx_l eleft >>= fun tau_l ->
       typecheck_expr ctx_r eright >>= fun tau_r ->
       if aequiv tau_l tau_r then Ok tau_l
       else err "case branches have different types: %s and %s"
           (Type.to_string tau_l) (Type.to_string tau_r)
     | _ -> err "Cannot case on non-sum of type %s" (Type.to_string tau_e))

  | Expr.Fix {x; tau; e} ->
    let ctx' = Map.set ctx ~key:x ~data:tau in
    typecheck_expr ctx' e >>= fun tau_e ->
    if aequiv tau tau_e then Ok tau
    else err "fix body type %s does not match declared type %s"
        (Type.to_string tau_e) (Type.to_string tau)

  | Expr.TyLam {a; e} ->
    typecheck_expr ctx e >>= fun tau ->
    Ok (Type.Forall {a; tau})

  | Expr.TyApp {e; tau} ->
    typecheck_expr ctx e >>= fun tau_e ->
    (match tau_e with
     | Type.Forall {a; tau = tau_body} ->
       Ok (Ast_util.Type.substitute a tau tau_body)
     | _ -> err "Cannot type-apply non-universal of type %s"
         (Type.to_string tau_e))

  | Expr.Fold_ {e; tau} ->
    (match tau with
     | Type.Rec {a; tau = tau_body} ->
       typecheck_expr ctx e >>= fun tau_e ->
       let unrolled = Ast_util.Type.substitute a tau tau_body in
       if aequiv tau_e unrolled then Ok tau
       else err "fold body type %s does not match unrolled type %s"
           (Type.to_string tau_e) (Type.to_string unrolled)
     | _ -> err "fold annotation is not a recursive type: %s"
         (Type.to_string tau))

  | Expr.Unfold e ->
    typecheck_expr ctx e >>= fun tau_e ->
    (match tau_e with
     | Type.Rec {a; tau = tau_body} ->
       Ok (Ast_util.Type.substitute a tau_e tau_body)
     | _ -> err "Cannot unfold non-recursive value of type %s"
         (Type.to_string tau_e))

  | Expr.Export {e; tau_adt; tau_mod} ->
    (match tau_mod with
     | Type.Exists {a; tau = tau_body} ->
       typecheck_expr ctx e >>= fun tau_e ->
       let expected = Ast_util.Type.substitute a tau_adt tau_body in
       if aequiv tau_e expected then Ok tau_mod
       else err "export body type %s does not match %s"
           (Type.to_string tau_e) (Type.to_string expected)
     | _ -> err "export annotation is not an existential type: %s"
         (Type.to_string tau_mod))

  | Expr.Import {x; a; e_mod; e_body} ->
    typecheck_expr ctx e_mod >>= fun tau_mod ->
    (match tau_mod with
     | Type.Exists {a = a_bound; tau = tau_body} ->
       (* Open the package: the witness type becomes the abstract variable [a],
          and [x] is bound to the module's representation type. *)
       let tau_x = Ast_util.Type.substitute a_bound (Type.Var a) tau_body in
       let ctx' = Map.set ctx ~key:x ~data:tau_x in
       typecheck_expr ctx' e_body
     | _ -> err "Cannot import from non-existential of type %s"
         (Type.to_string tau_mod))

let typecheck t = typecheck_expr String.Map.empty t

let inline_tests () =
  let open Poly in
  let p_ex = Parser.parse_expr_exn in
  let p_ty = Parser.parse_type_exn in
  let e1 = p_ex "fun (x : num) -> x" in
  assert (typecheck e1 = Ok(p_ty "num -> num"));

  let e2 = p_ex "fun (x : num) -> y" in
  assert (Result.is_error (typecheck e2));

  let t3 = p_ex "(fun (x : num) -> x) 3"in
  assert (typecheck t3 = Ok(p_ty "num"));

  let t4 = p_ex "((fun (x : num) -> x) 3) 3" in
  assert (Result.is_error (typecheck t4));

  let t5 = p_ex "0 + (fun (x : num) -> x)" in
  assert (Result.is_error (typecheck t5))

(* Uncomment the line below when you want to run the inline tests. *)
(* let () = inline_tests () *)
