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

(* I still suspect I am iterating it wrong, better to check it once*)
let state_to_string (state : state) : string =
  let slistList = List.mapi state.grid ~f:( fun (r :int) (row : cell list) ->
	List.mapi row ~f:(fun (col :int) (c : cell) -> 
									  let (x,y) = state.karel_pos in
									  if ( col = x && r = y) then "K"
									  else 
										if c = Empty then "."
										else if c = Wall then "x"
										else "B"
									  )) in
  let slist = List.map slistList ~f:( fun (row : string list) ->
  String.concat ~sep:" " row) in
  String.concat ~sep:"\n" slist
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


let rec eval_pred (state : state) (pred : predicate) : bool =
	match pred with
	| Facing dir -> if (state.karel_dir = dir) then true else false
	| Not pred' -> not (eval_pred state pred') 
	| NoBeepersPresent -> 
	  let (col,row) = state.karel_pos in
			if get_cell state.grid (col, row) <> Some Beeper then true else false
	| FrontIs c ->
	  let (col,row) = state.karel_pos in
		let front = if state.karel_dir = North then get_cell state.grid (col, row-1)
					else if state.karel_dir = South then get_cell state.grid (col, row + 1)
					else if state.karel_dir = West then get_cell state.grid (col-1, row)
					else get_cell state.grid (col+1, row) 
		in
		(match front with 
		| None -> true
		| Some x -> if (c = x) then true else false)
		(*else if (c = Empty && front = Empty) then true else false)
		if (c = Wall && front = Some Wall) then true
		if (c = Empty && front = Some Empty) then true *)
  


let rec step (state : state) (code : instruction) : state =
  match code with
  | Move ->
  	  (let (col,row) = state.karel_pos in
		let front = if state.karel_dir = North then get_cell state.grid (col, row-1)
					else if state.karel_dir = South then get_cell state.grid (col, row + 1)
					else if state.karel_dir = West then get_cell state.grid (col-1, row)
					else get_cell state.grid (col+1, row) 
		in
		(match front with 
		| None -> state
		| Some x -> if ((x = Wall)) then state
					else
						if state.karel_dir = North then {state with karel_pos = (col, row-1)}
						else if state.karel_dir = South then {state with karel_pos = (col, row+1)}
						else if state.karel_dir = West then {state with karel_pos = (col-1, row)}
						else {state with karel_pos = (col+1, row)}))
			
  | TurnLeft ->
	if state.karel_dir = North then {state with karel_dir = West}
	else if state.karel_dir = West then {state with karel_dir = South}
	else if state.karel_dir = South then {state with karel_dir = East}
	else {state with karel_dir = North}
  | PickBeeper ->
	let (col,row) = state.karel_pos in
		let curr_cell = get_cell state.grid (col,row) in
			if (curr_cell = Some Beeper) then {state with grid = (set_cell state.grid (col, row) Empty)} else state
  | PutBeeper ->
	let (col,row) = state.karel_pos in
		let curr_cell = get_cell state.grid (col,row) in
			if (curr_cell <> Some Beeper) then {state with grid = (set_cell state.grid (col, row) Beeper)} else state
  | While (pred, instrs) ->
	if ((eval_pred state pred) = true) then step_list (state) (instrs @ [While (pred, instrs)]) else state
  | If (pred, then_, else_) ->
		if ((eval_pred state pred) = true ) then List.fold then_ ~init:state ~f:(fun state instr -> step state instr)
		else List.fold else_ ~init:state ~f:(fun state instr -> step state instr)

	

  
  

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

(* If ((Facing East), [Move], [PutBeeper])
 While ((Not (FrontIs Wall)) , [PutBeeper; Move])
 
 To decide if last block will get Beeper or not it might be goot to come back a step and check if it already has Beeper and take the next steps
 *)
let checkers_algo : instruction list = [
While ((Not (FrontIs Wall)) , [
While ((Not (FrontIs Wall)) ,
[
 PutBeeper; Move; Move;	
]
);

TurnLeft; TurnLeft;
Move;
If ((NoBeepersPresent), [ TurnLeft; TurnLeft; Move; PutBeeper], [TurnLeft; TurnLeft; Move]);
TurnLeft; TurnLeft; TurnLeft;

If ((Not (FrontIs Wall)),
[
Move;
TurnLeft; TurnLeft; TurnLeft;

While ((Not (FrontIs Wall)) ,
[
 Move;	
]
);

TurnLeft; TurnLeft;

While ((Not (FrontIs Wall)) ,
[
 Move; PutBeeper; Move;	
]
);

TurnLeft; TurnLeft;
Move;
If ((NoBeepersPresent), [ TurnLeft; TurnLeft; Move; PutBeeper], [TurnLeft; TurnLeft; Move]);
TurnLeft; TurnLeft; TurnLeft;

If ((Not (FrontIs Wall)), 
[
Move;
TurnLeft; TurnLeft; TurnLeft;

While ((Not (FrontIs Wall)) ,
[
 Move;	
]
);

TurnLeft; TurnLeft;
], []);

], 
[]);

]);

If ((Facing East), [TurnLeft; TurnLeft] , [If ((Facing North), [TurnLeft], [ If ((Facing South), [TurnLeft; TurnLeft; TurnLeft], [])]) ]);
While ((Not (FrontIs Wall)) ,
[
 Move;	
]
);
]
