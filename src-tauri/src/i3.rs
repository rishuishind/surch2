use i3ipc::I3Connection;
use crate::models::SearchResultItem;

pub fn get_i3_items() -> Vec<SearchResultItem> {
    let mut results = Vec::new();
    let mut conn = match I3Connection::connect() {
        Ok(c) => c,
        Err(_) => return results, // Not running i3
    };

    // 1. Fetch Workspaces
    if let Ok(workspaces) = conn.get_workspaces() {
        for ws in workspaces.workspaces {
            results.push(SearchResultItem {
                id: format!("i3-ws-{}", ws.name),
                title: format!("Workspace {}", ws.name),
                subtitle: Some("Switch i3 Workspace".to_string()),
                icon: None,
                icon_data: None, // Frontend can render an i3 icon
                item_type: "i3_command".to_string(),
                action_data: format!("workspace \"{}\"", ws.name),
                score: 0,
            });
        }
    }

    // 2. Fetch Windows recursively from tree
    if let Ok(tree) = conn.get_tree() {
        fn find_windows(node: &i3ipc::reply::Node, results: &mut Vec<SearchResultItem>) {
            // A node is typically a window if it has a window ID or name
            if let Some(name) = &node.name {
                if node.window.is_some() {
                    let id_str = format!("i3-win-{}", node.id);
                    results.push(SearchResultItem {
                        id: id_str.clone(),
                        title: name.clone(),
                        subtitle: Some("Switch to Window".to_string()),
                        icon: None,
                        icon_data: None,
                        item_type: "i3_command".to_string(),
                        // Using con_id is the most reliable way to target the exact container
                        action_data: format!("[con_id=\"{}\"] focus", node.id),
                        score: 0,
                    });
                }
            }
            // Traverse down
            for child in &node.nodes {
                find_windows(child, results);
            }
            for floating in &node.floating_nodes {
                find_windows(floating, results);
            }
        }
        find_windows(&tree, &mut results);
    }

    results
}
