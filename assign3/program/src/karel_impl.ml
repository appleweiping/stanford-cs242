open Core
open Option.Monad_infix

exception Unimplemented

(* Set this to true to print out intermediate state between Karel steps *)
let debug = false

type cell =
  | Empty
  | Wall
  | Beeper

type grid = cell list list

type dir =
  | North
  | West
  | South
  | East

type pos = int * int

type state = {
  karel_pos : pos;
  karel_dir : dir;
  grid : grid;
}

let get_cell (grid : grid) ((i, j) : pos) : cell option =
  (List.nth grid j) >>= fun l -> List.nth l i
;;

let set_cell (grid : grid) ((i, j) : pos) (cell : cell) : grid =
  List.mapi grid ~f:(fun j' l ->
    if j = j' then List.mapi l ~f:(fun i' c -> if i = i' then cell else c)
    else l)
;;

let dir_to_char (d : dir) : char =
  match d with
  | North -> '^'
  | South -> 'v'
  | East -> '>'
  | West -> '<'
;;

(* Render the grid row by row. Karel is drawn as a direction arrow at its
   position; otherwise each cell is '.', 'W', or 'B'. *)
let state_to_string (state : state) : string =
  let (ki, kj) = state.karel_pos in
  let rows =
    List.mapi state.grid ~f:(fun j row ->
      let cells =
        List.mapi row ~f:(fun i cell ->
          if i = ki && j = kj then dir_to_char state.karel_dir
          else match cell with
            | Empty -> '.'
            | Wall -> 'W'
            | Beeper -> 'B')
      in
      String.of_char_list cells)
  in
  String.concat ~sep:"\n" rows
;;

let empty_grid (m : int) (n : int) : grid =
  List.map (List.range 0 m) ~f:(fun _ ->
    List.map (List.range 0 n) ~f:(fun _ -> Empty))
;;

type predicate =
  | FrontIs of cell
  | NoBeepersPresent
  | Facing of dir
  | Not of predicate

type instruction =
  | Move
  | TurnLeft
  | PickBeeper
  | PutBeeper
  | While of predicate * instruction list
  | If of predicate * instruction list * instruction list

let rec predicate_to_string (pred : predicate) : string =
  match pred with
  | FrontIs c ->
    let cellstr = match c with
      | Empty -> "Empty" | Beeper -> "Beeper" | Wall -> "Wall"
    in
    Printf.sprintf "FrontIs(%s)" cellstr
  | NoBeepersPresent -> "NoBeepersPresent"
  | Facing dir ->
    let dirstr = match dir with
      | North -> "North" | South -> "South" | East -> "East" | West -> "West"
    in
    Printf.sprintf "Facing(%s)" dirstr
  | Not pred' -> Printf.sprintf "Not(%s)" (predicate_to_string pred')

let rec instruction_to_string (instr : instruction) : string =
  match instr with
  | Move -> "Move"
  | TurnLeft -> "TurnLeft"
  | PickBeeper -> "PickBeeper"
  | PutBeeper -> "PutBeeper"
  | While (pred, instrs) ->
    Printf.sprintf "While(%s, [%s])"
      (predicate_to_string pred)
      (instruction_list_to_string instrs)
  | If (pred, then_, else_) ->
    Printf.sprintf "If(%s, [%s], [%s])"
      (predicate_to_string pred)
      (instruction_list_to_string then_)
      (instruction_list_to_string else_)
and instruction_list_to_string (instrs: instruction list) : string =
  String.concat ~sep:", " (List.map ~f:instruction_to_string instrs)


(* Unit delta (di, dj) for a direction. Row index j grows downward, so North
   decreases j and South increases j. *)
let dir_delta (d : dir) : int * int =
  match d with
  | North -> (0, -1)
  | South -> (0, 1)
  | East -> (1, 0)
  | West -> (-1, 0)
;;

(* The cell directly in front of Karel. *)
let front_pos (state : state) : pos =
  let (i, j) = state.karel_pos in
  let (di, dj) = dir_delta state.karel_dir in
  (i + di, j + dj)
;;

(* What is in front of Karel? Off-grid counts as a Wall. *)
let front_cell (state : state) : cell =
  match get_cell state.grid (front_pos state) with
  | Some c -> c
  | None -> Wall
;;

let rec eval_pred (state : state) (pred : predicate) : bool =
  match pred with
  | FrontIs c ->
    (match (front_cell state, c) with
     | (Empty, Empty) | (Wall, Wall) | (Beeper, Beeper) -> true
     | _ -> false)
  | NoBeepersPresent ->
    (match get_cell state.grid state.karel_pos with
     | Some Beeper -> false
     | _ -> true)
  | Facing d ->
    (match (state.karel_dir, d) with
     | (North, North) | (South, South) | (East, East) | (West, West) -> true
     | _ -> false)
  | Not pred' -> not (eval_pred state pred')
;;

let turn_left (d : dir) : dir =
  match d with
  | North -> West
  | West -> South
  | South -> East
  | East -> North
;;

let rec step (state : state) (code : instruction) : state =
  match code with
  | Move ->
    (* Move forward one cell unless a wall blocks the way. *)
    (match front_cell state with
     | Wall -> state
     | _ -> { state with karel_pos = front_pos state })
  | TurnLeft ->
    { state with karel_dir = turn_left state.karel_dir }
  | PickBeeper ->
    (match get_cell state.grid state.karel_pos with
     | Some Beeper ->
       { state with grid = set_cell state.grid state.karel_pos Empty }
     | _ -> state)
  | PutBeeper ->
    { state with grid = set_cell state.grid state.karel_pos Beeper }
  | While (pred, body) ->
    if eval_pred state pred
    then step (step_list state body) (While (pred, body))
    else state
  | If (pred, then_, else_) ->
    if eval_pred state pred
    then step_list state then_
    else step_list state else_

and step_list (state : state) (instrs : instruction list) : state =
  List.fold instrs ~init:state ~f:(fun state instr ->
    if debug then
       (Printf.printf "Executing instruction %s...\n"
          (instruction_to_string instr);
        let state' = step state instr in
        Printf.printf "Executed instruction %s. New state:\n%s\n"
          (instruction_to_string instr)
          (state_to_string state');
        state')
     else
       step state instr)

;;

(* Problem 3 - checkerboard.
   Karel starts at (0,0) facing East. We lay beepers so that every cell with
   (i + j) even carries a beeper: a checkerboard anchored at the origin, on an
   arbitrary m x n grid.

   Rather than tracking column/row parity (impossible without state), we use a
   local rule that provably yields a checkerboard:

     - Bottom row (row 0): place a beeper on the current cell, hop two cells,
       repeat. This fills the even columns of row 0 (beeper at column 0).
     - Every higher row: a cell gets a beeper iff the cell directly BELOW it
       has none. Karel can inspect the cell below by facing South and testing
       FrontIs Beeper. This "opposite-of-below" rule makes every column
       alternate vertically, which — combined with the checkered bottom row —
       produces the full checkerboard.

   Karel snakes up the grid: checker a row heading East, climb North, checker
   the next row heading West (using the same below-rule, direction-agnostic),
   climb North, and so on until the North wall stops the climb.

   Uses only Move / TurnLeft / PickBeeper / PutBeeper / While / If and the four
   predicates. *)

(* Rotate to face a specific absolute heading by turning left until we do.
   Bounded (at most 3 turns) since there are 4 headings. *)
let face (d : dir) : instruction list = [ While (Not (Facing d), [ TurnLeft ]) ]

(* Go to the wall in the current heading. *)
let goto_wall : instruction list = [ While (Not (FrontIs Wall), [ Move ]) ]

(* Row 0: heading East, place a beeper on the current cell and every second
   cell up to the East wall. *)
let checker_bottom_row : instruction list = [
  PutBeeper;
  While (Not (FrontIs Wall), [
    Move;
    If (Not (FrontIs Wall), [ Move; PutBeeper ], [ ]);
  ]);
]

(* The grid is stored top-row-first, and Karel starts at the top-left origin
   (0,0). The open direction into the rest of the grid is therefore SOUTH
   (increasing row index), and the already-checkered neighbour of an upper row
   is the cell to the NORTH. *)

(* At the current cell (heading East), place a beeper iff the cell directly to
   the NORTH (the previously-checkered row) is empty. Faces North to inspect,
   then restores the East heading. *)
let place_opposite_of_north : instruction list = [
  TurnLeft;                                 (* East -> North *)
  If (Not (FrontIs Beeper),
      [ TurnLeft; TurnLeft; TurnLeft; PutBeeper ]  (* North -> East, place *)
    , [ TurnLeft; TurnLeft; TurnLeft ]);           (* North -> East, no place *)
]

(* Checker a lower row heading East using the north-neighbour rule. *)
let checker_lower_row_east : instruction list =
  place_opposite_of_north
  @ [ While (Not (FrontIs Wall),
        [ Move ] @ place_opposite_of_north) ]

let checkers_algo : instruction list =
  face East
  @ checker_bottom_row
  (* Repeatedly: return to the West wall facing East, descend South one row if
     possible, and checker that row with the north-neighbour rule. When there
     is no row below, leave Karel facing South so the `Facing East` loop guard
     becomes false and the program terminates. *)
  @ [
    While (Facing East, (
      face West @ goto_wall @ face East
      @ [
        TurnLeft; TurnLeft; TurnLeft;        (* East -> South (look down) *)
        If (Not (FrontIs Wall),
            (* room below: descend and checker heading East *)
            [ Move ] @ face East @ checker_lower_row_east,
            (* no room: leave Karel facing South -> guard ends *)
            [ ]);
      ]))
  ]
