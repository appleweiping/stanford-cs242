open Core

exception Unimplemented

type ngram = string list
type ngram_map = (ngram, string list) Map.Poly.t
type word_distribution = float String.Map.t

(* Helpers for the skeleton's inline asserts, which compare structured values.
   Core shadows the polymorphic (=) with int equality, so we spell out the
   comparisons on string lists explicitly. *)
let seq (a : string list) (b : string list) = List.equal String.equal a b
let seqseq (a : string list list) (b : string list list) =
  List.equal (List.equal String.equal) a b

(* remove_last: drop the last element of a list.
   Implementation 1 - recursive, pattern matching. *)
let rec remove_last_impl1 (l : string list) : string list =
  match l with
  | [] -> []
  | [_] -> []
  | x :: rest -> x :: remove_last_impl1 rest
;;

assert (seq (remove_last_impl1 ["a"; "b"]) ["a"]);
;;

(* Implementation 2 - using library combinators. Reverse, drop the head,
   reverse back. *)
let remove_last_impl2 (l : string list) : string list =
  match List.rev l with
  | [] -> []
  | _ :: rest -> List.rev rest
;;

assert (seq (remove_last_impl2 ["a"; "b"]) ["a"])
;;

(* compute_ngrams l n : every contiguous sublist (window) of length n. *)
let compute_ngrams (l : string list) (n : int) : string list list =
  if n <= 0 then []
  else
    let rec aux l =
      let window = List.take l n in
      if List.length window < n then []
      else window :: aux (List.tl_exn l)
    in
    aux l
;;

assert (seqseq (compute_ngrams ["a"; "b"; "c"] 2) [["a"; "b"]; ["b"; "c"]]);
;;

let ngram_to_string ng =
  Printf.sprintf "[%s]" (String.concat ~sep:", " ng)
;;

let ngram_map_new () : ngram_map =
  Map.Poly.empty
;;

(* ngram_map_add : record one n-gram. The first (n-1) words form the key
   (the context/prefix); the last word is a successor observed after that
   context. We accumulate successors (with multiplicity) so distributions can
   later reflect frequency. *)
let ngram_map_add (map : ngram_map) (ngram : ngram) : ngram_map =
  match List.rev ngram with
  | [] -> map
  | last :: rev_prefix ->
    let prefix = List.rev rev_prefix in
    Map.Poly.update map prefix ~f:(function
      | None -> [last]
      | Some successors -> last :: successors)
;;

let () =
  let map = ngram_map_new () in
  let map = ngram_map_add map ["a"; "b"] in
  (* After adding ["a"; "b"], the key ["a"] maps to the successors ["b"]. *)
  assert (match Map.Poly.find map ["a"] with
          | Some succ -> seq succ ["b"]
          | None -> false);
  ()
;;

(* ngram_map_distribution : for a given context, return the empirical
   probability distribution over successor words (count / total). *)
let ngram_map_distribution (map : ngram_map) (ngram : ngram)
  : word_distribution option =
  match Map.Poly.find map ngram with
  | None -> None
  | Some successors ->
    let total = Float.of_int (List.length successors) in
    let counts =
      List.fold successors ~init:String.Map.empty ~f:(fun acc w ->
        Map.update acc w ~f:(function None -> 1 | Some c -> c + 1))
    in
    Some (Map.map counts ~f:(fun c -> Float.of_int c /. total))
;;

let distribution_to_string (dist : word_distribution) : string =
  Sexp.to_string_hum (String.Map.sexp_of_t Float.sexp_of_t dist)
;;

(* sample_distribution : draw a word from the distribution using inverse-CDF
   sampling on a uniform [0,1) draw. *)
let sample_distribution (dist : word_distribution) : string =
  let r = Random.float 1.0 in
  let rec pick acc = function
    | [] ->
      (* Shouldn't happen for a proper distribution; return last word if any. *)
      (match Map.to_alist dist with
       | [] -> raise (Failure "sample_distribution: empty distribution")
       | l -> fst (List.last_exn l))
    | (word, p) :: rest ->
      let acc' = acc +. p in
      if Float.(r < acc') then word else pick acc' rest
  in
  pick 0.0 (Map.to_alist dist)
;;

(* sample_n : generate n words. Starting from the context ngram [ng], look up
   its distribution, sample the next word, emit it, and slide the context
   window forward (drop first word of ng, append the sampled word). If a
   context has no successors, stop early. *)
let rec sample_n (map : ngram_map) (ng : ngram) (n : int) : string list =
  if n <= 0 then []
  else
    match ngram_map_distribution map ng with
    | None -> []
    | Some dist ->
      let next = sample_distribution dist in
      let ng' =
        match ng with
        | [] -> [next]
        | _ :: rest -> rest @ [next]
      in
      next :: sample_n map ng' (n - 1)
;;
