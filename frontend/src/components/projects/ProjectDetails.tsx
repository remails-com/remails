import {StreamsOverview} from "../streams/StreamsOverview.tsx";
import {useProjects} from "../../hooks/useProjects.ts";
import {Loader} from "../../Loader.tsx";
import {useForm} from '@mantine/form';
import {Button, Grid, Group, Stack, TextInput, Tooltip} from "@mantine/core";
import {Project} from "../../types.ts";
import {modals} from "@mantine/modals";
import {notifications} from "@mantine/notifications";
import {IconTrash, IconX} from "@tabler/icons-react";
import {useCurrentOrganization} from "../../hooks/useCurrentOrganization.ts";
import {useRemails} from "../../hooks/useRemails.ts";
import {useStreams} from "../../hooks/useStreams.ts";
import {useEffect, useState} from "react";


interface FormValues {
  name: string,
}

export function ProjectDetails() {
  const currentOrganisation = useCurrentOrganization();
  const [canDelete, setCanDelete] = useState<boolean>(false);
  const {currentProject} = useProjects();
  const {streams} = useStreams();
  const {dispatch, navigate} = useRemails();

  useEffect(() => {
    if (streams && streams.length === 0) {
      setCanDelete(true)
    } else {
      setCanDelete(false)
    }
  }, [streams]);

  const form = useForm<FormValues>({
    mode: 'controlled',
    onSubmitPreventDefault: 'always',
    initialValues: {
      name: currentProject?.name || ""
    },
  });

  if (!currentProject || !currentOrganisation) {
    return <Loader/>;
  }

  const confirmDeleteProject = (project: Project) => {
    modals.openConfirmModal({
      title: 'Please confirm your action',
      children: `Are you sure you want to delete project ${project.name}? This action cannot be undone`,
      labels: {confirm: 'Confirm', cancel: 'Cancel'},
      onCancel: () => {
      },
      onConfirm: () => deleteProject(project),
    });
  }

  const deleteProject = (project: Project) => {
    fetch(`/api/organizations/${currentOrganisation.id}/projects/${project.id}`, {
      method: 'DELETE',
    }).then(res => {
      if (res.status === 200) {
        notifications.show({
          title: 'Project deleted',
          message: `Project ${project.name} deleted`,
          color: 'green',
        })
        dispatch({type: "remove_project", projectId: project.id})
        navigate('projects')
      } else {
        notifications.show({
          title: 'Error',
          message: `Project ${project.name} could not be deleted`,
          color: 'red',
          autoClose: 20000,
          icon: <IconX size={20}/>,
        })
        console.error(res)
      }
    })
  }

  const save = (values: FormValues) => {
    form.resetDirty()
    notifications.show({
      title: "Not yet implemented",
      message: "You found me",
      color: 'red'
    })
    return new Promise((resolve) => setTimeout(() => resolve(values), 500));
  }

  return (
    <Grid gutter="xl">
      <Grid.Col span={{base: 12, md: 6, lg: 3}}>
        <h2>Project Details</h2>
        <form onSubmit={form.onSubmit(save)}>
          <Stack>
            <TextInput
              label="Name"
              key={form.key('name')}
              value={form.values.name}
              onChange={(event) => form.setFieldValue('name', event.currentTarget.value)}
            />
            <Group>
              <Tooltip label={canDelete ? 'Delete project' : 'Cannot delete project, there are streams in it'}>
                <Button leftSection={<IconTrash/>}
                        color="red"
                        disabled={!canDelete}
                        onClick={() => confirmDeleteProject(currentProject)}>Delete</Button>
              </Tooltip>
              <Button type="submit" disabled={!form.isDirty()} loading={form.submitting}>Save</Button>
            </Group>
          </Stack>
        </form>
      </Grid.Col>
      <Grid.Col span={{base: 12, md: 6, lg: 9}}>
        <h2>Streams</h2>
        <StreamsOverview/>
      </Grid.Col>
    </Grid>

  )
}
