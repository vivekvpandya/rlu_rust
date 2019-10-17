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

(* for fold_left style can we check lenght of accumulater to take any useful decisions *)

let split xs size =
  let (_, r, rs) =
    (* fold over the list, keeping track of how many elements are still
       missing in the current list (csize), the current list (ys) and
       the result list (zss) *) 
    List.fold_left (fun (csize, ys, zss) elt ->
      (* if target size is 0, add the current list to the target list and
         start a new empty current list of target-size size *)
      if csize = 0 then (size - 1, [elt], zss @ [ys])
      (* otherwise decrement the target size and append the current element
         elt to the current list ys *)
      else (csize - 1, ys @ [elt], zss))
      (* start the accumulator with target-size=size, an empty current list and
         an empty target-list *)
        (size, [], []) xs
  in
  (* add the "left-overs" to the back of the target-list *)
  rs @ [r]
  
  
      List.fold_left ["A"; "b"; "c"] (2, [], [])  (fun (csize, ys, zss) elt ->
      if csize = 0 then (size - 1, [elt], zss @ [ys])
      else (csize - 1, ys @ [elt], zss))
	  
	  
	  
	  let () =
  let map = ngram_map_new () in
  let map = ngram_map_add map ["a"; "b"] in
  let v = match Map.Poly.find map ["a"] with
		  | None -> []
		  | Some x -> x
  in
  v
  ()
;;


let li = ["C"; "C"; "A"] in
let len : int = List.length li in
let nlist = List.map ~f:(fun (s: string) -> (s, ((float_of_int 1) /. (float_of_int len)))) li in
let prob = String.Map.of_alist_reduce nlist (+.) in
String.Map.find prob "C";;

This variant expression is expected to have type unit
       The constructor true does not belong to type unit
	   
	   
	   
	   let checkers_algo : instruction list = [
While ((Not (FrontIs Wall)) ,
[
	[While ((Not (FrontIs Wall)) , [PutBeeper; Move; Move]);
	 PutBeeper; TurnLeft; TurnLeft; TurnLeft;
	 If ((FrontIs Wall), [], [Move; TurnLeft; TurnLeft; TurnLeft]);
	 ]

]
)
]