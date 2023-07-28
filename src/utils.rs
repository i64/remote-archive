pub struct TreeNode {
    name: String,
    children: Vec<TreeNode>,
}

pub fn build_tree(raw_tree: Vec<String>) -> TreeNode {
    let mut root_node = TreeNode {
        name: "/".to_string(),
        children: Vec::new(),
    };

    raw_tree
        .iter()
        .map(|p| p.split('/').collect())
        .for_each(|pv| build_subtree(&mut root_node, pv));

    root_node
}

fn build_subtree(node: &mut TreeNode, parts: Vec<&str>) {
    if let Some(part) = parts.first() {
        if !part.is_empty() {
            let mut found_child = None;
            for (index, child) in node.children.iter_mut().enumerate() {
                if &child.name == part {
                    found_child = Some(index);
                    break;
                }
            }

            if let Some(index) = found_child {
                build_subtree(&mut node.children[index], parts[1..].to_vec());
            } else {
                let new_node = TreeNode {
                    name: part.to_string(),
                    children: Vec::new(),
                };
                node.children.push(new_node);
                build_subtree(node.children.last_mut().unwrap(), parts[1..].to_vec());
            }
        }
    }
}

pub fn display_tree(node: &TreeNode, depth: usize) {
    let indentation = "│   ".repeat(depth);
    let node_name = if depth > 0 {
        format!("{}├── {}", indentation, node.name)
    } else {
        node.name.clone()
    };
    println!("{}", node_name);

    for child in &node.children {
        display_tree(child, depth + 1);
    }
}
