let main () =
 
  let rec print_list_string (myList : string list) (start : bool) = 
  match myList with
  | [] -> Printf.printf " ]"
  | head::body -> 
  begin
  if start then Printf.printf "[ " ;
  Printf.printf "%s; " head;
  print_list_string body false;
  end
  in

  let rec extract (n : int) (l : string list) : string list list=
	let len : int = List.length l in
	if n > len || n = 0 then [ [] ]
	else match l with
		 | [] -> [ [] ]
		 | h :: t -> [ [h] @ (List.nth (extract (n-1) t) 0) ] @ (extract n t)
  in
  List.iter (fun ll -> print_list_string ll true) (extract 3 ["a"; "b"; "c"; "d"]);
  Printf.printf "\n";
  List.iter (fun ll -> print_list_string ll true) (extract 2 ["a"; "b"; "c"; "d"]);
  Printf.printf "\n";
  List.iter (fun ll -> print_list_string ll true) (extract 1 ["a"; "b"; "c"; "d"]);
  (* print_list_string (extract 2 ["a"; "b"; "c"]); *)
  (* need to define one more function*)
  ()
  

let () = main () 
