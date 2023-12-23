use std::collections::BTreeMap;

#[derive(Clone)]
pub struct Node<T> {
    pub name: String,
    pub value: Option<T>,
    pub children: BTreeMap<String, Node<T>>,
}

impl<T> Node<T> {
    pub fn new(name: String, value: Option<T>) -> Self {
        Node {
            name,
            value,
            children: BTreeMap::new(),
        }
    }

    pub fn add_node(&mut self, name: &str, value: Option<T>) {
        let parts = name.split('.');
        let mut current_node = self;

        for part in parts {
            if !current_node.children.contains_key(part) {
                let new_node = Node::new(part.to_string(), None);
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let node = Node::new("root".to_string(), Some(1));

        assert_eq!(node.name, "root");
        assert_eq!(node.value, Some(1));
        assert!(node.children.is_empty());
    }

    #[test]
    fn test_get_node() {
        let mut root = Node::new("root".to_string(), None);

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
        let mut root = Node::new("root".to_string(), None);

        root.add_node("aws.s3.bucket", Some("abc".to_string()));
        root.add_node("aws.s3.key", Some("xyz".to_string()));
        root.add_node("aws.region", Some("us-east-1".to_string()));

        let aws_s3 = root.get_node("aws.s3").unwrap();
        assert!(aws_s3.children.contains_key("bucket"));
        assert!(aws_s3.children.contains_key("key"));
        assert!(!aws_s3.children.contains_key("nonexistent"));
    }
}
