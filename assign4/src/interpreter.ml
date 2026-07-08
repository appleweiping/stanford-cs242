open Lam_flags
open Core
open Ast

type outcome =
  | Step of Expr.t
  | Val

exception RuntimeError of string

let rec trystep (e : Expr.t) : outcome =
  match e with
  | (Expr.Lam _ | Expr.Num _ | Expr.True | Expr.False | Expr.Pair _ | Expr.Unit
    | Expr.Inject _ | Expr.TyLam _ | Expr.Export _ | Expr.Fold_ _) -> Val

  | Expr.Binop {binop; left; right} ->
    (left, fun left' -> Expr.Binop {left = left'; binop; right;}) |-> fun () ->
    (right, (fun right' -> Expr.Binop {right = right'; binop; left})) |-> fun () ->
    let (Expr.Num n1, Expr.Num n2) = (left, right) in
    let f = match binop with
      | Expr.Add -> (+)
      | Expr.Sub -> (-)
      | Expr.Mul -> ( * )
      | Expr.Div -> (/)
    in
    Step (Expr.Num (f n1 n2) )

  | Expr.Relop {relop; left; right} ->
    (left, fun left' -> Expr.Relop {relop; left = left'; right}) |-> fun () ->
    (right, fun right' -> Expr.Relop {relop; left; right = right'}) |-> fun () ->
    let (Expr.Num n1, Expr.Num n2) = (left, right) in
    let b = match relop with
      | Expr.Lt -> n1 < n2
      | Expr.Gt -> n1 > n2
      | Expr.Eq -> n1 = n2
    in
    Step (if b then Expr.True else Expr.False)

  | Expr.And {left; right} ->
    (left, fun left' -> Expr.And {left = left'; right}) |-> fun () ->
    (match left with
     | Expr.True -> Step right
     | Expr.False -> Step Expr.False
     | _ -> raise (RuntimeError "&& applied to non-boolean"))

  | Expr.Or {left; right} ->
    (left, fun left' -> Expr.Or {left = left'; right}) |-> fun () ->
    (match left with
     | Expr.True -> Step Expr.True
     | Expr.False -> Step right
     | _ -> raise (RuntimeError "|| applied to non-boolean"))

  | Expr.If {cond; then_; else_} ->
    (cond, fun cond' -> Expr.If {cond = cond'; then_; else_}) |-> fun () ->
    (match cond with
     | Expr.True -> Step then_
     | Expr.False -> Step else_
     | _ -> raise (RuntimeError "if on non-boolean"))

  | Expr.App {lam; arg} ->
    (lam, fun lam' -> Expr.App {lam = lam'; arg}) |-> fun () ->
    (arg, fun arg' -> Expr.App {lam; arg = arg'}) |-> fun () ->
    (match lam with
     | Expr.Lam {x; e; _} -> Step (Ast_util.Expr.substitute x arg e)
     | _ -> raise (RuntimeError "application of non-function"))

  | Expr.Project {e; d} ->
    (e, fun e' -> Expr.Project {e = e'; d}) |-> fun () ->
    (match e with
     | Expr.Pair {left; right} ->
       Step (match d with Expr.Left -> left | Expr.Right -> right)
     | _ -> raise (RuntimeError "projection of non-pair"))

  | Expr.Case {e; xleft; eleft; xright; eright} ->
    (e, fun e' -> Expr.Case {e = e'; xleft; eleft; xright; eright}) |-> fun () ->
    (match e with
     | Expr.Inject {e = v; d = Expr.Left; _} ->
       Step (Ast_util.Expr.substitute xleft v eleft)
     | Expr.Inject {e = v; d = Expr.Right; _} ->
       Step (Ast_util.Expr.substitute xright v eright)
     | _ -> raise (RuntimeError "case on non-injection"))

  | Expr.Fix {x; tau; e} ->
    (* Unroll: fix x. e  |->  e[fix x. e / x]  (substitute the whole fix,
       not just its body, so recursive references keep unrolling). *)
    Step (Ast_util.Expr.substitute x (Expr.Fix {x; tau; e}) e)

  | Expr.TyApp {e; tau} ->
    (e, fun e' -> Expr.TyApp {e = e'; tau}) |-> fun () ->
    (match e with
     | Expr.TyLam {e = body; _} -> Step body
     | _ -> raise (RuntimeError "type application of non-type-abstraction"))

  | Expr.Unfold e ->
    (e, fun e' -> Expr.Unfold e') |-> fun () ->
    (match e with
     | Expr.Fold_ {e = v; _} -> Step v
     | _ -> raise (RuntimeError "unfold of non-fold"))

  | Expr.Import {x; a; e_mod; e_body} ->
    (e_mod, fun e_mod' -> Expr.Import {x; a; e_mod = e_mod'; e_body})
    |-> fun () ->
    (match e_mod with
     | Expr.Export {e = v; _} -> Step (Ast_util.Expr.substitute x v e_body)
     | _ -> raise (RuntimeError "import of non-export"))

  | Expr.Var _ ->
    raise (RuntimeError (
      Printf.sprintf "Reached a stuck state at expression: %s"
        (Expr.to_string e)))

  | _ -> raise (RuntimeError (
    Printf.sprintf "Reached a stuck state at expression: %s" (Expr.to_string e)))

and (|->) ((e, hole) : Expr.t * (Expr.t -> Expr.t)) (next : unit -> outcome)
  : outcome =
  match trystep e with Step e' -> Step (hole e') | Val -> next ()

let rec eval e =
  match trystep e with
  | Step e' ->
    (if extra_verbose () then
       Printf.printf "Stepped:\n%s\n|->\n%s\n\n"
         (Expr.to_string e) (Expr.to_string e'));
    eval e'
  | Val -> Ok e

let inline_tests () =
  let open Poly in
  let p = Parser.parse_expr_exn in
  let e1 = p "2 + 3" in
  assert (trystep e1 = Step(Expr.Num 5));

  let e2 = p "(fun (x : num) -> x) 3" in
  assert (trystep e2 = Step(Expr.Num 3))

(* Uncomment the line below when you want to run the inline tests. *)
(* let () = inline_tests () *)
