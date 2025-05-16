import {Button, Group, Modal, Stack, TextInput} from '@mantine/core';
import {useForm} from "@mantine/form";
import {useOrganizations} from "../../hooks/useOrganizations.ts";
import {useRemails} from "../../hooks/useRemails.ts";
import {IconX} from "@tabler/icons-react";
import { notifications } from '@mantine/notifications';

interface FormValues {
  name: string,
}

interface NewProjectProps {
  opened: boolean;
  close: () => void;
}

export function NewProject({opened, close}: NewProjectProps) {
  const {currentOrganization} = useOrganizations();
  const {navigate, dispatch} = useRemails();

  const form = useForm<FormValues>({
    initialValues: {
      name: ""
    },
    validate: {
      name: (value) => (value.length < 3 ? 'Name must have at least 3 letters' : null),
    }
  });

  if (!currentOrganization) {
    return <></>
  }

  const save = (values: FormValues) => {
    fetch(`/api/organizations/${currentOrganization.id}/projects`, {
      method: 'POST',
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(values)
    }).then(res => {
      if (res.status === 201) {
        close()
        res.json().then(newProject => {
          dispatch({type: "add_project", project: newProject})
          navigate('projects.project', {proj_id: newProject.id})
          notifications.show({
            title: 'Project created',
            message: `Project ${newProject.name} created`,
            color: 'green',
          })
        })
      } else if (res.status === 409) {
        form.setFieldError('name', 'Project with this name already exists')
        return
      } else {
        notifications.show({
          title: 'Error',
          message: 'Something went wrong',
          color: 'red',
          autoClose: 20000,
          icon: <IconX size={20}/>,
        })
      }
    })
  }

  return (
    <>
      <Modal opened={opened} onClose={close} title="Create New Project">
        <form onSubmit={form.onSubmit(save)}>
          <Stack>
          <TextInput
            data-autofocus
            label="Name"
            key={form.key('name')}
            value={form.values.name}
            placeholder="New project"
            error={form.errors.name}
            onChange={(event) => form.setFieldValue('name', event.currentTarget.value)}
          />
          </Stack>

          <Group justify="space-between" mt="xl">
            <Button onClick={close} variant="outline">Cancel</Button>
            <Button type="submit" loading={form.submitting}>Save</Button>
          </Group>
        </form>
      </Modal>
    </>
  );
}