import {useCurrentOrganization} from "../../hooks/useCurrentOrganization.ts";
import {useStreams} from "../../hooks/useStreams.ts";
import {useProjects} from "../../hooks/useProjects.ts";
import {useCredentials} from "../../hooks/useCredentials.ts";
import {useRemails} from "../../hooks/useRemails.ts";
import {useForm} from "@mantine/form";
import {useEffect} from "react";
import {Loader} from "../../Loader.tsx";
import {SmtpCredential} from "../../types.ts";
import {modals} from "@mantine/modals";
import {Button, Grid, Group, Stack, Text, Textarea, TextInput, Tooltip} from "@mantine/core";
import {notifications} from "@mantine/notifications";
import {IconTrash, IconX} from "@tabler/icons-react";

interface FormValues {
  username: string,
  description: string,
}

export function CredentialDetails() {
  const currentOrganisation = useCurrentOrganization();
  const {currentStream} = useStreams();
  const {currentProject} = useProjects();
  const {currentCredential} = useCredentials();
  const {dispatch, navigate} = useRemails();

  const form = useForm<FormValues>();

  useEffect(() => {
    form.setValues({
      username: currentCredential?.username || "",
      description: currentCredential?.description
    });
    form.resetDirty();
  }, [currentCredential]);

  if (!currentStream || !currentOrganisation || !currentProject || !currentCredential) {
    return <Loader/>;
  }

  const confirmDeleteCredential = (credential: SmtpCredential) => {
    modals.openConfirmModal({
      title: 'Please confirm your action',
      children: (
        <Text>
          Are you sure you want to delete the SMTP credential with the username <strong>{credential.username}</strong>?
          You won't be able to sent messages with this credential anymore. This action cannot be undone.
        </Text>
      ),
      labels: {confirm: 'Confirm', cancel: 'Cancel'},
      onCancel: () => {
      },
      onConfirm: () => deleteCredential(credential),
    });
  }

  const deleteCredential = (credential: SmtpCredential) => {
    fetch(`/api/organizations/${currentOrganisation.id}/projects/${currentProject.id}/streams/${currentStream.id}/smtp_credentials/${credential.id}`, {
      method: 'DELETE',
    }).then(res => {
      if (res.status === 200) {
        notifications.show({
          title: 'Credential deleted',
          message: `Credential with username ${credential.username} deleted`,
          color: 'green',
        })
        dispatch({type: "remove_credential", credentialId: credential.id})
        navigate('projects.project.streams.stream')
      } else {
        notifications.show({
          title: 'Error',
          message: `Credential with username ${credential.username} could not be deleted`,
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
        <h2>SMTP credential Details</h2>
        <form onSubmit={form.onSubmit(save)}>
          <Stack>
            <TextInput
              variant='filled'
              label="Username"
              key={form.key('name')}
              value={form.values.username}
              readOnly
            />
            <Textarea
            label="Description"
            autosize
            maxRows={10}
            key={form.key('name')}
            value={form.values.description}
            onChange={(event) => form.setFieldValue('description', event.currentTarget.value)}
            />
            <Tooltip label='The password cannot be shown or changed. Please create a new credential if needed and possibly delete this one.'>
            <TextInput
              label='Password'
              value='••••••••'
              readOnly
              variant='filled'
            />
            </Tooltip>
            <Group>
              <Tooltip label='Delete SMTP credential'>
                <Button leftSection={<IconTrash/>}
                        color="red"
                        onClick={() => confirmDeleteCredential(currentCredential)}>Delete</Button>
              </Tooltip>
              <Button type="submit" disabled={!form.isDirty()} loading={form.submitting}>Save</Button>
            </Group>
          </Stack>
        </form>
      </Grid.Col>
      {/*<Grid.Col span={{base: 12, md: 6, lg: 9}}>*/}
      {/*  <h2>Credentials</h2>*/}
      {/*  <CredentialsOverview/>*/}
      {/*  <h2>Messages</h2>*/}
      {/*  <MessageLog/>*/}
      {/*</Grid.Col>*/}
    </Grid>
  )
}