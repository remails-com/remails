import { Button, Group, Stack, TextInput } from "@mantine/core";
import { useRemails } from "../../hooks/useRemails.ts";
import { useForm } from "@mantine/form";
import { notifications } from "@mantine/notifications";
import { Organization } from "../../types.ts";
import { errorNotification } from "../../notify.tsx";

interface FormValues {
  name: string;
}

interface NewOrganizationFormProps {
  done: (newOrg: Organization) => void;
  close: () => void;
}

export async function saveNewOrganization(name: string): Promise<{ status: number; newOrg: Organization | null }> {
  const res = await fetch(`/api/organizations`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ name }),
  });

  if (res.status === 201) {
    return res.json().then((newOrg) => {
      notifications.show({
        title: "Organization created",
        message: `Organization ${newOrg.name} created`,
        color: "green",
      });

      return { status: 201, newOrg };
    });
  }

  errorNotification(`Organization ${name} could not be created`);
  console.error(res);

  return { status: res.status, newOrg: null };
}

export function NewOrganizationForm({ done, close }: NewOrganizationFormProps) {
  const { dispatch, state } = useRemails();
  const user = state.user!;

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
        done(newOrg);
        dispatch({
          type: "set_user",
          user: { ...user, org_roles: [...user.org_roles, { role: "read_only", org_id: newOrg.id }] },
        });
        dispatch({ type: "add_organization", organization: newOrg });
      }
    });
  };

  return (
    <form onSubmit={form.onSubmit(save)}>
      <Stack>
        <TextInput
          label="Name"
          key={form.key("name")}
          value={form.values.name}
          placeholder="New organization"
          error={form.errors.name}
          onChange={(event) => form.setFieldValue("name", event.currentTarget.value)}
        />
        <Group justify="space-between" mt="lg">
          <Button onClick={close} variant="outline">
            Cancel
          </Button>
          <Button type="submit" loading={form.submitting}>
            Save
          </Button>
        </Group>
      </Stack>
    </form>
  );
}
