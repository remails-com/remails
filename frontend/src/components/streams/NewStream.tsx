import {useOrganizations} from "../../hooks/useOrganizations.ts";
import {useRemails} from "../../hooks/useRemails.ts";
import {useForm} from "@mantine/form";
import {useProjects} from "../../hooks/useProjects.ts";
import {notifications} from "@mantine/notifications";
import {IconX} from "@tabler/icons-react";
import {useStreams} from "../../hooks/useStreams.ts";
import {Button, Group, Modal, Stack, TextInput} from "@mantine/core";

interface FormValues {
  name: string,
}

interface NewStreamProps {
  opened: boolean;
  close: () => void;
}

export function NewStream({opened, close}: NewStreamProps) {
  const {currentOrganization} = useOrganizations();
  const {currentProject} = useProjects();
  const {streams} = useStreams();
  const {navigate, dispatch} = useRemails();

  const form = useForm<FormValues>({
    initialValues: {
      name: ""
    },
    validate: {
      name: (value) => {
        if (value.length < 3) {
          return 'Name must have at least 3 letters';
        }
        if (streams?.find((s) => s.name === value)) {
          return 'Stream with this name already exists';
        }
        return null;
      },
    }
  });

  if (!currentOrganization || !currentProject) {
    console.error("Cannot create stream without a selected organization and project")
    return <></>
  }

  const save = (values: FormValues) => {
    fetch(`/api/organizations/${currentOrganization.id}/projects/${currentProject.id}/streams`, {
      method: 'POST',
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(values)
    }).then(res => {
      if (res.status === 201) {
        close()
        res.json().then(newStream => {
          dispatch({type: "add_stream", stream: newStream})
          navigate('projects.project.streams.stream', {stream_id: newStream.id})
          notifications.show({
            title: 'Stream created',
            message: `Stream ${newStream.name} created`,
            color: 'green',
          })
        })
      } else if (res.status === 409) {
        form.setFieldError('name', 'Stream with this name already exists')
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
      <Modal opened={opened} onClose={close} title="Create New Stream">
        <form onSubmit={form.onSubmit(save)}>
          <Stack>
            <TextInput
              label="Name"
              key={form.key('name')}
              value={form.values.name}
              placeholder="New Stream"
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