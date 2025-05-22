import { Button, Group, Modal, Stack, Text, TextInput } from "@mantine/core";
import { saveNewOrganization } from "./organizations/NewOrganizationForm.tsx";
import { useForm } from "@mantine/form";
import { useRemails } from "../hooks/useRemails.ts";

interface SetupProps {
  opened: boolean;
  close: () => void;
}

interface FormValues {
  name: string;
}

export function Setup({ opened, close }: SetupProps) {
  const { navigate, dispatch } = useRemails();

  const form = useForm<FormValues>({
    initialValues: {
      name: "",
    },
    validate: {
      name: (value) => (value.length < 3 ? "Name must have at least 3 letters" : null),
    },
  });

  const save = (values: FormValues) => {
    saveNewOrganization(values.name).then(({ status, newOrg }) => {
      if (status === 201 && newOrg) {
        close();
        dispatch({ type: "add_organization", organization: newOrg });
        navigate("projects", { org_id: newOrg.id });
      }
    });
  };

  return (
    <Modal
      opened={opened}
      onClose={() => {}}
      withCloseButton={false}
      centered
      overlayProps={{ backgroundOpacity: 0.55, blur: 3 }}
    >
      <Text size="lg" fw={500}>
        Welcome to Rem@ils
      </Text>
      <form onSubmit={form.onSubmit(save)}>
        <Stack mt="lg">
          <Text>To get started with Remails, please create a new Organization</Text>
          <TextInput
            label="Name"
            key={form.key("name")}
            value={form.values.name}
            placeholder="New organization"
            error={form.errors.name}
            onChange={(event) => form.setFieldValue("name", event.currentTarget.value)}
          />
          <Group justify="flex-end" mt="lg">
            <Button type="submit" loading={form.submitting}>
              Create Organization
            </Button>
          </Group>
        </Stack>
      </form>
    </Modal>
  );
}
