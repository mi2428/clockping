pub(crate) fn validate_metric_prefix(prefix: &str) -> anyhow::Result<()> {
    if !is_valid_metric_name(prefix) {
        anyhow::bail!("invalid Prometheus metric prefix '{prefix}'");
    }
    Ok(())
}

pub(crate) fn is_valid_label_name(name: &str) -> bool {
    is_valid_metric_name(name)
}

pub(super) fn validate_prometheus_labels(labels: &[(String, String)]) -> anyhow::Result<()> {
    for (name, value) in labels {
        if !is_valid_label_name(name) {
            anyhow::bail!("invalid Prometheus label name '{name}'");
        }
        if value.is_empty() {
            anyhow::bail!("Prometheus label value for '{name}' must not be empty");
        }
    }
    reject_duplicate_labels(labels)
}

pub(crate) fn reject_duplicate_labels(labels: &[(String, String)]) -> anyhow::Result<()> {
    for (index, (name, _)) in labels.iter().enumerate() {
        if labels[..index]
            .iter()
            .any(|(previous_name, _)| previous_name == name)
        {
            anyhow::bail!("duplicate Prometheus label name '{name}'");
        }
    }
    Ok(())
}

fn is_valid_metric_name(name: &str) -> bool {
    let Some((&first, rest)) = name.as_bytes().split_first() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return false;
    }
    rest.iter()
        .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
}
