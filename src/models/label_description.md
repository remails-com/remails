Labels are used to mark email and ease filtering. They are normalized with the following
rules to reduce the risk of having multiple almost identical labels and ambiguities.

## Normalization rules

* All space, tab or newline characters at the start or end are removed
* All space, tab, newline, comma (,) characters within the label are replaced by a dash (-).
  If multiple of those characters directly following each other, they are replaced with only a
  single dash.
* All characters are made lowercase

**Note:** After normalization, the label must be at least one and at most 30 characters long.