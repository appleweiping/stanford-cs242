open Lam_flags
open Core

exception Unimplemented

let fresh s = s ^ "'"

module Type = struct
  open Ast.Type

  let rec substitute_map (rename : t String.Map.t) (tau : t) : t =
    match tau with
    | Num -> Num
    | Bool -> Bool
    | Unit -> Unit
    | Var x ->
      (match Map.find rename x with
       | Some tau' -> tau'
       | None -> Var x)
    | Fn {arg; ret} ->
      Fn {arg = substitute_map rename arg; ret = substitute_map rename ret}
    | Product {left; right} ->
      Product {left = substitute_map rename left;
               right = substitute_map rename right}
    | Sum {left; right} ->
      Sum {left = substitute_map rename left;
           right = substitute_map rename right}
    (* Binders: alpha-rename the bound variable to a fresh name so that (a) it
       cannot capture free variables coming from the substituted types, and
       (b) it correctly shadows any mapping for the same name. *)
    | Rec {a; tau} ->
      let a' = fresh a in
      let rename' = Map.set rename ~key:a ~data:(Var a') in
      Rec {a = a'; tau = substitute_map rename' tau}
    | Forall {a; tau} ->
      let a' = fresh a in
      let rename' = Map.set rename ~key:a ~data:(Var a') in
      Forall {a = a'; tau = substitute_map rename' tau}
    | Exists {a; tau} ->
      let a' = fresh a in
      let rename' = Map.set rename ~key:a ~data:(Var a') in
      Exists {a = a'; tau = substitute_map rename' tau}

  let substitute (x : string) (tau' : t) (tau : t) : t =
    substitute_map (String.Map.singleton x tau') tau

  let rec to_debruijn (tau : t) : t =
    (* Convert to a nameless representation for alpha-equivalence checking.
       Each bound variable is replaced by "$" ^ (de Bruijn index), i.e. the
       number of binders between the variable and its binder. Free variables
       keep their name. Binder names are erased (set to "$"). *)
    let rec aux (depth : int String.Map.t) (tau : t) : t =
      let enter a = Map.set (Map.map depth ~f:(fun d -> d + 1))
                      ~key:a ~data:0 in
      match tau with
      | Num -> Num
      | Bool -> Bool
      | Unit -> Unit
      | Var x ->
        (match Map.find depth x with
         | Some i -> Var ("$" ^ string_of_int i)
         | None -> Var x)
      | Fn {arg; ret} -> Fn {arg = aux depth arg; ret = aux depth ret}
      | Product {left; right} ->
        Product {left = aux depth left; right = aux depth right}
      | Sum {left; right} ->
        Sum {left = aux depth left; right = aux depth right}
      | Rec {a; tau} -> Rec {a = "$"; tau = aux (enter a) tau}
      | Forall {a; tau} -> Forall {a = "$"; tau = aux (enter a) tau}
      | Exists {a; tau} -> Exists {a = "$"; tau = aux (enter a) tau}
    in
    aux String.Map.empty tau

  let rec aequiv (tau1 : t) (tau2 : t) : bool =
    let rec aux (tau1 : t) (tau2 : t) : bool =
      match (tau1, tau2) with
      | (Num, Num) -> true
      | (Bool, Bool) | (Unit, Unit) -> true
      | (Var x, Var y) -> String.equal x y
      | (Fn x, Fn y) -> aux x.arg y.arg && aux x.ret y.ret
      | (Sum x, Sum y) -> aux x.left y.left && aux x.right y.right
      | (Product x, Product y) -> aux x.left y.left && aux x.right y.right
      | (Rec x, Rec y) -> aux x.tau y.tau
      | (Forall x, Forall y) -> aux x.tau y.tau
      | (Exists x, Exists y) -> aux x.tau y.tau
      | _ -> false
    in
    aux (to_debruijn tau1) (to_debruijn tau2)

  let inline_tests () =
    let p = Parser.parse_type_exn in

    assert (
      aequiv
        (substitute "a" (p "num") (p "forall b . a"))
        (p "forall a . num"));
    assert (
      aequiv
        (substitute "a" (p "b") (p "forall b . a"))
        (p "forall c . b"));
    assert (
      not (aequiv
        (substitute "a" (p "b") (p "forall b . a"))
        (p "forall b . b")));
    assert (
      aequiv
        (substitute "a" (p "b") (p "forall b . forall b . a"))
        (p "forall q . forall c . b"));
    assert (
      not (aequiv
        (substitute "a" (p "b") (p "forall b . forall b . a"))
        (p "forall a . forall b . a")));

    assert (aequiv (p "forall a . a") (p "forall b . b"));
    assert (not (aequiv (p "forall a . a") (p "forall b . num")));
    assert (aequiv
              (p "forall a . forall b . a -> b")
              (p "forall x . forall y . x -> y"))

  (* Uncomment the line below when you want to run the inline tests. *)
  (* let () = inline_tests () *)
end

module Expr = struct
  open Ast.Expr

  let rec substitute_map (rename : t String.Map.t) (e : t) : t =
    (* Capture-avoiding parallel substitution of *term* variables. Type
       annotations (tau) and type-variable binders (TyLam / Import's [a]) are
       left untouched; only term variables are substituted. Whenever we enter
       a term binder we alpha-rename it to a fresh name to avoid capture. *)
    let sub = substitute_map in
    let bind rename x =
      let x' = fresh x in
      (x', Map.set rename ~key:x ~data:(Var x'))
    in
    match e with
    | Num _ -> e
    | Binop {binop; left; right} ->
      Binop {binop; left = sub rename left; right = sub rename right}
    | True | False | Unit -> e
    | If {cond; then_; else_} ->
      If {cond = sub rename cond; then_ = sub rename then_;
          else_ = sub rename else_}
    | Relop {relop; left; right} ->
      Relop {relop; left = sub rename left; right = sub rename right}
    | And {left; right} -> And {left = sub rename left; right = sub rename right}
    | Or {left; right} -> Or {left = sub rename left; right = sub rename right}
    | Var x ->
      (match Map.find rename x with Some e' -> e' | None -> Var x)
    | Lam {x; tau; e} ->
      let (x', rename') = bind rename x in
      Lam {x = x'; tau; e = sub rename' e}
    | App {lam; arg} -> App {lam = sub rename lam; arg = sub rename arg}
    | Pair {left; right} ->
      Pair {left = sub rename left; right = sub rename right}
    | Project {e; d} -> Project {e = sub rename e; d}
    | Inject {e; d; tau} -> Inject {e = sub rename e; d; tau}
    | Case {e; xleft; eleft; xright; eright} ->
      let (xleft', rl) = bind rename xleft in
      let (xright', rr) = bind rename xright in
      Case {e = sub rename e;
            xleft = xleft'; eleft = sub rl eleft;
            xright = xright'; eright = sub rr eright}
    | Fix {x; tau; e} ->
      let (x', rename') = bind rename x in
      Fix {x = x'; tau; e = sub rename' e}
    | TyLam {a; e} -> TyLam {a; e = sub rename e}
    | TyApp {e; tau} -> TyApp {e = sub rename e; tau}
    | Fold_ {e; tau} -> Fold_ {e = sub rename e; tau}
    | Unfold e -> Unfold (sub rename e)
    | Export {e; tau_adt; tau_mod} ->
      Export {e = sub rename e; tau_adt; tau_mod}
    | Import {x; a; e_mod; e_body} ->
      let (x', rename') = bind rename x in
      Import {x = x'; a; e_mod = sub rename e_mod; e_body = sub rename' e_body}

  let substitute (x : string) (e' : t) (e : t) : t =
    substitute_map (String.Map.singleton x e') e

  let rec to_debruijn (e : t) : t =
    (* Erase *term*-variable names, replacing each with "$" ^ index. Matches
       the structure compared by aequiv (which ignores type annotations). *)
    let rec aux (depth : int String.Map.t) (e : t) : t =
      let enter x = Map.set (Map.map depth ~f:(fun d -> d + 1))
                      ~key:x ~data:0 in
      match e with
      | Num _ -> e
      | Binop {binop; left; right} ->
        Binop {binop; left = aux depth left; right = aux depth right}
      | True | False | Unit -> e
      | If {cond; then_; else_} ->
        If {cond = aux depth cond; then_ = aux depth then_;
            else_ = aux depth else_}
      | Relop {relop; left; right} ->
        Relop {relop; left = aux depth left; right = aux depth right}
      | And {left; right} -> And {left = aux depth left; right = aux depth right}
      | Or {left; right} -> Or {left = aux depth left; right = aux depth right}
      | Var x ->
        (match Map.find depth x with
         | Some i -> Var ("$" ^ string_of_int i)
         | None -> Var x)
      | Lam {x; tau; e} -> Lam {x = "$"; tau; e = aux (enter x) e}
      | App {lam; arg} -> App {lam = aux depth lam; arg = aux depth arg}
      | Pair {left; right} ->
        Pair {left = aux depth left; right = aux depth right}
      | Project {e; d} -> Project {e = aux depth e; d}
      | Inject {e; d; tau} -> Inject {e = aux depth e; d; tau}
      | Case {e; xleft; eleft; xright; eright} ->
        Case {e = aux depth e;
              xleft = "$"; eleft = aux (enter xleft) eleft;
              xright = "$"; eright = aux (enter xright) eright}
      | Fix {x; tau; e} -> Fix {x = "$"; tau; e = aux (enter x) e}
      | TyLam {a; e} -> TyLam {a; e = aux depth e}
      | TyApp {e; tau} -> TyApp {e = aux depth e; tau}
      | Fold_ {e; tau} -> Fold_ {e = aux depth e; tau}
      | Unfold e -> Unfold (aux depth e)
      | Export {e; tau_adt; tau_mod} ->
        Export {e = aux depth e; tau_adt; tau_mod}
      | Import {x; a; e_mod; e_body} ->
        Import {x = "$"; a; e_mod = aux depth e_mod;
                e_body = aux (enter x) e_body}
    in
    aux String.Map.empty e

  let aequiv (e1 : t) (e2 : t) : bool =
    let rec aux (e1 : t) (e2 : t) : bool =
      match (e1, e2) with
      | (Num n1, Num n2) -> n1 = n2
      | (Var x, Var y) -> String.equal x y
      | (Binop l, Binop r) ->
        Poly.equal l.binop r.binop && aux l.left r.left && aux l.right r.right
      | (True, True) | (False, False) -> true
      | (If l, If r) ->
        aux l.cond r.cond && aux l.then_ r.then_ && aux l.else_ r.else_
      | (Relop l, Relop r) ->
        Poly.equal l.relop r.relop && aux l.left r.left && aux l.right r.right
      | (And l, And r) ->
        aux l.left r.left && aux l.right r.right
      | (Or l, Or r) ->
        aux l.left r.left && aux l.right r.right
      | (Lam l, Lam r) ->
        aux l.e r.e
      | (App l, App r) ->
        aux l.lam r.lam && aux l.arg r.arg
      | (Unit, Unit) -> true
      | (Pair l, Pair r) ->
        aux l.left r.left && aux l.right r.right
      | (Project l, Project r) ->
        aux l.e r.e && Poly.equal l.d r.d
      | (Inject l, Inject r) ->
        aux l.e r.e && Poly.equal l.d r.d
      | (Case l, Case r) ->
        aux l.e r.e && aux l.eleft r.eleft && aux l.eright r.eright
      | (Fix l, Fix r) -> aux l.e r.e
      | (TyLam l, TyLam r) ->
        aux l.e r.e
      | (TyApp l, TyApp r) -> aux l.e r.e
      | (Fold_ l, Fold_ r) -> aux l.e r.e
      | (Unfold l, Unfold r) -> aux l r
      | (Export l, Export r) -> aux l.e r.e
      | (Import l, Import r) -> aux l.e_mod r.e_mod && aux l.e_body r.e_body
      | _ -> false
    in
    aux (to_debruijn e1) (to_debruijn e2)

  let inline_tests () =
    let p = Parser.parse_expr_exn in
    let t1 = p "(fun (x : num) -> x) y" in
    assert (aequiv (substitute "x" (Num 0) t1) t1);
    assert (aequiv (substitute "y" (Num 0) t1)
              (p "(fun (x : num) -> x) 0"));

    let t2 = p "x + (fun (x : num) -> y)" in
    assert (aequiv
              (substitute "x" (Num 0) t2)
              (p "0 + (fun (x : num) -> y)"));
    assert (aequiv (substitute "y" (Num 0) t2)
              (p "x + (fun (x : num) -> 0)"));

    assert (aequiv (p "fun (x : num) -> x") (p "fun (y : num) -> y"));

    assert (not (aequiv (p "fun (x : num) -> fun (x : num) -> x + x")
                   (p "fun (x : num) -> fun (y : num) -> y + x")));

    assert (
      aequiv
        (p "tyfun a -> fun (x : a) -> x")
        (p "tyfun b -> fun (x : b) -> x"));

    ()

  (* Uncomment the line below when you want to run the inline tests. *)
  (* let () = inline_tests () *)
end
