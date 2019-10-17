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
  
	let rec k_list (l : string list) (k : int) : string list =
		let len : int = List.length l in
		if len < k then []
		else match k with
		 |0 -> []
		 |_ -> List.hd l :: (k_list (List.tl l) (k-1))
	in
	let rec compute_ngrams (l : string list) (n : int) : string list list =
		let len : int = List.length l in
		if len < n then [ ]
		else match n with
		  |0 -> [ [] ]
		  |_ -> [ k_list l n ] @ (compute_ngrams (List.tl l) n)
	in
	(* print_list_string (k_list ["a"; "b"; "c"; "d"] 2) true; *)
	List.iter (fun ll -> print_list_string ll true) (compute_ngrams ["a"; "b"; "c"] 2);
	assert (compute_ngrams ["a"; "b"; "c"] 2 = [["a"; "b"]; ["b"; "c"]]);
	()
let () = main () 