use polars_utils::arena::{Arena, Node};
use polars_utils::pl_str::PlSmallStr;

use crate::prelude::IR;

pub(super) fn optimize(root: Node, lp_arena: &mut Arena<IR>) {
    let mut stack = vec![root];
    let mut inputs = vec![];

    while let Some(node) = stack.pop() {
        match lp_arena.get(node) {
            IR::MergeSorted { key, .. } => {
                let key = key.clone();

                inputs.clear();
                collect_merge_sorted_inputs(node, key.as_str(), lp_arena, &mut inputs);

                if inputs.len() > 2 {
                    let rebuilt_ir = rebuild_merge_sorted_tree(&inputs, key, lp_arena);
                    lp_arena.replace(node, rebuilt_ir);
                }

                stack.extend(inputs.iter().copied());
            },
            ir => ir.copy_inputs(&mut stack),
        }
    }
}

fn collect_merge_sorted_inputs(root: Node, key: &str, lp_arena: &Arena<IR>, out: &mut Vec<Node>) {
    let mut stack = vec![root];

    while let Some(node) = stack.pop() {
        match lp_arena.get(node) {
            IR::MergeSorted {
                input_left,
                input_right,
                key: merge_key,
            } if merge_key.as_str() == key => {
                stack.push(*input_right);
                stack.push(*input_left);
            },
            _ => out.push(node),
        }
    }
}

fn rebuild_merge_sorted_tree(inputs: &[Node], key: PlSmallStr, lp_arena: &mut Arena<IR>) -> IR {
    let mut current_level = inputs.to_vec();

    while current_level.len() > 2 {
        let mut next_level = Vec::with_capacity(current_level.len().div_ceil(2));

        for pair in current_level.chunks(2) {
            match pair {
                [input_left, input_right] => {
                    next_level.push(lp_arena.add(IR::MergeSorted {
                        input_left: *input_left,
                        input_right: *input_right,
                        key: key.clone(),
                    }));
                },
                [input] => next_level.push(*input),
                _ => unreachable!(),
            }
        }

        current_level = next_level;
    }

    match current_level.as_slice() {
        [input_left, input_right] => IR::MergeSorted {
            input_left: *input_left,
            input_right: *input_right,
            key,
        },
        _ => unreachable!("rebuild_merge_sorted_tree requires at least 3 inputs"),
    }
}
