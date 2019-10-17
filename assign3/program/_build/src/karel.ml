open Core
open Karel_impl

let run_problem1 () =
  let problem1 : state = {
    karel_pos = (2, 2);
    karel_dir = East;
    grid = [
      [Empty; Empty; Empty; Empty; Empty; Empty; Empty];
      [Empty; Wall;  Wall;  Wall;  Wall;  Wall;  Empty];
      [Empty; Wall;  Empty; Empty; Empty; Wall;  Empty];
      [Empty; Wall;  Empty; Empty; Empty; Empty; Beeper];
      [Empty; Wall;  Wall;  Wall;  Wall;  Wall;  Empty];
      [Empty; Empty; Empty; Empty; Empty; Empty; Empty];
    ]
  } in

  let p1_algo = [
    Move; Move; TurnLeft; TurnLeft; TurnLeft;
    Move; TurnLeft;
    Move; Move; PickBeeper; TurnLeft; TurnLeft;
    Move; Move; Move; Move; TurnLeft; TurnLeft; TurnLeft;
    Move; TurnLeft; TurnLeft; TurnLeft;
  ] in

  Printf.printf "Initial state:\n%s\n\n" (state_to_string problem1);
  let final_state = step_list problem1 p1_algo in
  Printf.printf "Final state:\n%s\n" (state_to_string final_state)

let run_problem2 () =
    let problem2 : state = {
    karel_pos = (0, 1);
    karel_dir = North;
    grid = [[Empty; Empty; Wall]; [Empty; Wall; Wall]; [Empty; Beeper; Empty]]
  } in
    let p2_algo = [
    Move; Move; TurnLeft; TurnLeft; TurnLeft;
    Move; TurnLeft;
    Move; Move; PickBeeper; TurnLeft; TurnLeft;
    Move; Move; Move; Move; TurnLeft; TurnLeft; TurnLeft;
    Move; TurnLeft; TurnLeft; TurnLeft;
  ] in

  Printf.printf "Initial state:\n%s\n\n" (state_to_string problem2);
  let final_state = step_list problem2 p2_algo in
  Printf.printf "Final state:\n%s\n" (state_to_string final_state)
  
let run_problem3 m n =
  let problem3 : state = {
    karel_pos = (0, 0);
    karel_dir = East;
    grid = empty_grid m n;
  } in

  Printf.printf "Initial state:\n%s\n\n" (state_to_string problem3);
  let final_state = step_list problem3 checkers_algo in
  Printf.printf "Final state:\n%s\n" (state_to_string final_state)


let main () =
  let open Command.Param in
  let open Command.Let_syntax in
  Command.basic
    ~summary:"NGraml generator"
    [%map_open
      let problem = anon ("problem" %: string)
      and m = flag "m" (optional_with_default 6 int) ~doc:"width"
      and n = flag "n" (optional_with_default 6 int) ~doc:"height"
      in
      fun () ->
        if problem = "problem1" then run_problem1 ()
        else if problem = "problem3" then run_problem3 m n
		else if problem = "problem2" then run_problem2 ()
        else Printf.printf "Error: first argument must be 'problem1' or 'problem3'"
    ]
  |> Command.run



let () = main ()
