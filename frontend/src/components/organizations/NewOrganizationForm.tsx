import {Button, Group, Stack, TextInput} from "@mantine/core";
import {useRemails} from "../../hooks/useRemails.ts";
import {useForm} from "@mantine/form";
import {notifications} from "@mantine/notifications";
import {IconX} from "@tabler/icons-react";
import {Organization} from "../../types.ts";
import {useUser} from "../../hooks/useUser.ts";

interface FormValues {
  name: string,
}

interface NewOrganizationFormProps {
  done: (newOrg: Organization) => void;
}

export async function saveNewOrganization(name: string): Promise<{ status: number, newOrg: Organization | null }> {
  const res = await fetch(`/api/organizations`, {
    method: 'POST',
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({name})
  });
  if (res.status === 201) {
    return res.json().then(newOrg => {
      notifications.show({
        title: 'Organization created',
        message: `Organization ${newOrg.name} created`,
        color: 'green',
      });
      return Promise.resolve({status: 201, newOrg});
    });
  } else {
    notifications.show({
      title: 'Error',
      message: 'Something went wrong',
      color: 'red',
      autoClose: 20000,
      icon: <IconX size={20}/>,
    });
    return Promise.resolve({status: res.status, newOrg: null});
  }
}

export function NewOrganizationForm({done}: NewOrganizationFormProps) {
  const {dispatch} = useRemails();
  const {user, setUser} = useUser();

  const form = useForm<FormValues>({
    initialValues: {
      name: ""
    },
    validate: {
      name: (value) => (value.length < 3 ? 'Name must have at least 3 letters' : null),
    }
  });

  const save = (values: FormValues) => {
    saveNewOrganization(values.name)
      .then(({status, newOrg}) => {
        if (status === 201 && newOrg) {
          done(newOrg)
          setUser({...user, roles: [...user.roles, {type: "organization_admin", id: newOrg.id}]})
          dispatch({type: "add_organization", organization: newOrg})
        }
      })
  }


  return (
    <form onSubmit={form.onSubmit(save)}>
      <Stack>
        <TextInput
          label="Name"
          key={form.key('name')}
          value={form.values.name}
          placeholder="New organization"
          error={form.errors.name}
          onChange={(event) => form.setFieldValue('name', event.currentTarget.value)}
        />
        <Group justify="space-between" mt="lg">
          <Button onClick={close} variant="outline">Cancel</Button>
          <Button type="submit" loading={form.submitting}>Save</Button>
        </Group>
      </Stack>
    </form>
  )
}