open Core

exception Unimplemented

type ngram = string list
type ngram_map = (ngram, string list) Map.Poly.t
type word_distribution = float String.Map.t

(* PRINT LIST STRING *)
let rec print_list_string myList = match myList with
| [] -> print_endline "This is the end of the string list!"
| head::body -> 
begin
print_endline head;
print_list_string body
end
;;

let rec remove_last_impl1 (l : string list) : string list =
  match l with
  | [] -> []
  | x :: xs ->
    if xs = [] then []
    else x::(remove_last_impl1 xs)
;;

(*print_list_string (remove_last_impl1["a"; "b"; "c"]);
print_list_string (remove_last_impl1["a"; "b"]);
print_list_string (remove_last_impl1["a"]);*)
assert (remove_last_impl1 ["a"; "b"] = ["a"]);
(*print_list_string (remove_last_impl1[]);*)
;;

let remove_last_impl2 (l : string list) : string list =
  let len : int = List.length l in
  List.filteri l ~f:(fun (i : int) (s : string) : bool -> i <> (len-1))
;;

assert (remove_last_impl2 ["a"; "b"] = ["a"])


(*print_list_string (remove_last_impl2["a"; "b"; "c"]);
print_list_string (remove_last_impl2["a"; "b"]);

print_list_string (remove_last_impl2["a"]);

print_list_string (remove_last_impl2[]);*)
;;


let compute_ngrams (l : string list) (n : int) : string list list =
	let (_, (r : string list), (rs : string list list)) = 
		List.fold_left 	l ~init:(n, [], []) ~f:(fun ((csize : int), (ys : string list), (zss : string list list)) elt ->
			if csize = 0 then (0, elt :: (remove_last_impl2 ys), (List.rev ys) :: zss)
			else (csize - 1, elt::ys, zss))
			
	in
	(List.rev ((List.rev r) :: rs))
;;


assert (compute_ngrams ["a"; "b"; "c"] 2 = [["a"; "b"]; ["b"; "c"]]);
assert (compute_ngrams ["a"; "b"; "c"] 1 = [["a"]; ["b"]; ["c"]]);
assert (compute_ngrams ["a"; "b"; "c"; "d"] 3 = [["a"; "b"; "c"]; ["b"; "c"; "d"]]);
assert (compute_ngrams ["a"; "b"; "c"; "d"] 2 = [["a"; "b"]; ["b"; "c"]; ["c"; "d"]]);
assert (compute_ngrams ["a"; "b"; "c"; "d"; "e"] 4 =  [["a"; "b"; "c"; "d"]; ["b"; "c"; "d"; "e"]]);
assert (compute_ngrams ["a"; "c"; "b"; "d"] 2 = [["a"; "c"]; ["c"; "b"]; ["b"; "d"]]);
;;

let ngram_to_string ng =
  Printf.sprintf "[%s]" (String.concat ~sep:", " ng)
;;

let ngram_map_new () : ngram_map =
  Map.Poly.empty
;;

(* hopefully below works *)
let ngram_map_add (map : ngram_map) (ngram : ngram) : ngram_map =
  let len : int = List.length ngram in
  let (k, d) = List.split_n ngram (len -1) in
  match Map.Poly.find map k with
		  | None -> Map.Poly.set map ~key:k ~data:d
		  | Some x -> Map.Poly.set map ~key:k ~data:(x @ d)
;;

let () =
  let map = ngram_map_new () in
  let map = ngram_map_add map ["a"; "b"] in
  let map = ngram_map_add map ["A"; "B"; "C"] in
  let map = ngram_map_add map ["A"; "B"; "Vivek"] in
    let v = match Map.Poly.find map ["A"; "B"] with
		  | None -> []
		  | Some x -> x
	in
	(*Printf.printf "%s" (ngram_to_string v);*)
	()
;;

let ngram_map_distribution (map : ngram_map) (ngram : ngram)
  : word_distribution option =
  let v = match Map.Poly.find map ngram with
		  | None -> []
		  | Some x -> x
  in
let len : int = List.length v in
let nlist = List.map ~f:(fun (s: string) -> (s, ((float_of_int 1) /. (float_of_int len)))) v in
let prob = String.Map.of_alist_reduce nlist (+.) in
Some(prob)
;;

let distribution_to_string (dist : word_distribution) : string =
  Sexp.to_string_hum (String.Map.sexp_of_t Float.sexp_of_t dist)
;;

let sample_distribution (dist : word_distribution) : string =
  let ran : float = Random.float 1. in
  (*Printf.printf "%f" ran;*)
  let s : string = "" in
  let (r, rs) = String.Map.fold dist ~init:(ran, s) ~f:(fun ~key ~data (r, outs) ->
	if (r < data) then (1. , key)
	else (r -. data, outs))
  in
  rs
;;

let () =
  let map = ngram_map_new () in
  let map = ngram_map_add map ["a"; "b"] in
  let map = ngram_map_add map ["A"; "B"; "C"] in
  let map = ngram_map_add map ["A"; "B"; "C"] in
  let map = ngram_map_add map ["A"; "B"; "D"] in
  let map = ngram_map_add map ["A"; "B"; "Vivek"] in
    let v = match Map.Poly.find map ["A"; "B"] with
		  | None -> []
		  | Some x -> x
	in
  let dist = ngram_map_distribution map ["A"; "B"] in
  match dist with
  | None -> ()
  | Some x -> 
  let sdist = sample_distribution x in
  (*Printf.printf "%s" sdist;
  Printf.printf "%s" (distribution_to_string x);
	Printf.printf "%s" (ngram_to_string v);*)
	()
;;

let rec sample_n (map : ngram_map) (ng : ngram) (n : int) : string list =
  if n = 0 then []
  else match Map.Poly.find map ng with 
		| None -> []
		| Some x -> 
		let dist = (ngram_map_distribution map ng) in
		match dist with 
		| None -> []
		| Some x -> let s = sample_distribution x in s :: sample_n map ((remove_last_impl2 (List.rev ng)) @ [s]) (n-1)	
;;
