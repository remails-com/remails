import { ActionIcon, FocusTrap, Group, TextInput, ThemeIcon, Title, Tooltip } from "@mantine/core";
import { useClickOutside, useDisclosure } from "@mantine/hooks";
import { IconCheck, IconEdit, IconX } from "@tabler/icons-react";
import classes from "./Rename.module.css";
import { useForm } from "@mantine/form";

interface FormValues {
  name: string;
}

interface RenameProps {
  name: string;
  save: (values: { name: string }) => void;
}

export default function Rename({ name: initial, save }: RenameProps) {
  const form = useForm<FormValues>({
    initialValues: {
      name: initial,
    },
    validate: {
      name: (value) => {
        if (value.length < 3) {
          return "Name must have at least 3 characters";
        }
        if (value.length > 50) {
          return "Name must be less than 50 characters";
        }
        return null;
      },
    },
  });

  const [isEdit, { open: edit, close: stopEdit }] = useDisclosure(false);
  const ref = useClickOutside(() => {
    form.setFieldValue("name", initial);
    stopEdit();
  });

  if (isEdit) {
    return (
      <form
        onSubmit={(e) => {
          e.preventDefault();
          if (form.validate().hasErrors) {
            return;
          }
          form.onSubmit(save)();
          stopEdit();
        }}
      >
        <Group ref={ref} gap="xs" grow preventGrowOverflow={false} align="flex-start">
          <FocusTrap>
            <TextInput
              key={form.key("name")}
              size="xs"
              pt="1px"
              styles={{ input: { fontSize: "16px" } }}
              error={form.errors.name}
              value={form.values.name}
              radius="md"
              onChange={(e) => {
                form.setFieldValue("name", e.target.value);
              }}
            />
          </FocusTrap>
          <Tooltip label="Save name">
            <ActionIcon size="30" variant="filled" type="submit">
              <IconCheck />
            </ActionIcon>
          </Tooltip>
          <Tooltip label="Cancel">
            <ActionIcon
              size="30"
              variant="filled"
              onClick={() => {
                stopEdit();
                form.setFieldValue("name", initial);
              }}
            >
              <IconX />
            </ActionIcon>
          </Tooltip>
        </Group>
      </form>
    );
  }

  return (
    <Group className={classes.rename} bdrs="md" justify="space-between" onClick={edit}>
      <Title order={3}>{form.values.name}</Title>
      <ThemeIcon variant="transparent">
        <IconEdit className={classes.editIcon} />
      </ThemeIcon>
    </Group>
  );
}
