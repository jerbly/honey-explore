use askama::Template;
use std::collections::BTreeMap;

#[derive(Template, Clone)]
#[template(path = "tree.html")]
pub struct Node<T> {
    pub name: String,
    pub path: String,
    pub value: Option<T>,
    pub children: BTreeMap<String, Node<T>>,
}

impl<T> Node<T> {
    pub fn new(name: String, path: String, value: Option<T>) -> Self {
        Node {
            name,
            path,
            value,
            children: BTreeMap::new(),
        }
    }

    fn make_path(path: &str, part: &str) -> String {
        if path.is_empty() {
            part.to_owned()
        } else {
            format!("{path}.{part}")
        }
    }

    pub fn add_node(&mut self, name: &str, value: Option<T>) {
        let parts = name.split('.');
        let mut current_node = self;

        for part in parts {
            if !current_node.children.contains_key(part) {
                let new_node = Node::<T>::new(
                    part.to_string(),
                    Node::<T>::make_path(&current_node.path, part),
                    None,
                );
                current_node.children.insert(part.to_string(), new_node);
            }
            current_node = current_node.children.get_mut(part).unwrap();
        }

        current_node.value = value;
    }

    pub fn get_node(&self, name: &str) -> Option<&Node<T>> {
        let parts = name.split('.');
        let mut current_node = self;

        for part in parts {
            match current_node.children.get(part) {
                Some(child_node) => current_node = child_node,
                None => return None,
            }
        }

        Some(current_node)
    }

    pub fn has_grandchild(&self) -> bool {
        for child in self.children.values() {
            if !child.children.is_empty() {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let node = Node::new("root".to_string(), "".to_owned(), Some(1));

        assert_eq!(node.name, "root");
        assert_eq!(node.value, Some(1));
        assert!(node.children.is_empty());
    }

    #[test]
    fn test_get_node() {
        let mut root = Node::new("root".to_string(), "".to_owned(), None);

        root.add_node("aws.s3.bucket", Some("abc".to_string()));
        root.add_node("aws.s3.key", Some("xyz".to_string()));
        root.add_node("aws.region", Some("us-east-1".to_string()));

        assert_eq!(
            root.get_node("aws.s3.bucket").unwrap().value,
            Some("abc".to_string())
        );
        assert_eq!(
            root.get_node("aws.s3.key").unwrap().value,
            Some("xyz".to_string())
        );
        assert_eq!(
            root.get_node("aws.region").unwrap().value,
            Some("us-east-1".to_string())
        );
        assert!(root.get_node("aws.s3.nonexistent").is_none());
    }

    #[test]
    fn test_children_of_aws_s3() {
        let mut root = Node::new("root".to_string(), "".to_owned(), None);

        root.add_node("aws.s3.bucket", Some("abc".to_string()));
        root.add_node("aws.s3.key", Some("xyz".to_string()));
        root.add_node("aws.region", Some("us-east-1".to_string()));

        let aws_s3 = root.get_node("aws.s3").unwrap();
        assert!(aws_s3.children.contains_key("bucket"));
        assert!(aws_s3.children.contains_key("key"));
        assert!(!aws_s3.children.contains_key("nonexistent"));
    }
}
