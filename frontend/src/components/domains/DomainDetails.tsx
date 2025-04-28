import {useCurrentOrganization} from "../../hooks/useCurrentOrganization.ts";
import {useProjects} from "../../hooks/useProjects.ts";
import {useRemails} from "../../hooks/useRemails.ts";
import {useForm} from "@mantine/form";
import {useDomains} from "../../hooks/useDomains.ts";
import {Loader} from "../../Loader.tsx";
import {Domain} from "../../types.ts";
import {modals} from "@mantine/modals";
import {notifications} from "@mantine/notifications";
import {IconTrash, IconX} from "@tabler/icons-react";
import {Button, Grid, Group, Stack, Text, TextInput, Tooltip} from "@mantine/core";

interface FormValues {
  domain: string,
}

export function DomainDetails() {
  const currentOrganisation = useCurrentOrganization();
  const {currentProject} = useProjects();
  const {currentDomain} = useDomains();
  const {dispatch, navigate} = useRemails();

  const form = useForm<FormValues>({
    mode: 'controlled',
    onSubmitPreventDefault: 'always',
    initialValues: {
      domain: currentDomain?.domain || ""
    },
  });

  if (!currentDomain || !currentOrganisation) {
    return <Loader/>;
  }

  const confirmDeleteDomain = (domain: Domain) => {
    modals.openConfirmModal({
      title: 'Please confirm your action',
      children: (
        <Text>
          Are you sure you want to delete the domain <strong>{domain.domain}</strong>? This action cannot be undone
        </Text>
      ),
      labels: {confirm: 'Confirm', cancel: 'Cancel'},
      onCancel: () => {
      },
      onConfirm: () => deleteDomain(domain),
    });
  }

  const deleteDomain = (domain: Domain) => {
    let url = `/api/organizations/${currentOrganisation.id}/domains/${domain.id}`;
    if (currentProject) {
      url = `/api/organizations/${currentOrganisation.id}/projects/${currentProject.id}/domains/${domain.id}`;
    }

    fetch(url, {
      method: 'DELETE',
    }).then(res => {
      if (res.status === 200) {
        notifications.show({
          title: 'Domain deleted',
          message: `Domain ${domain.domain} deleted`,
          color: 'green',
        })
        dispatch({type: "remove_domain", domainId: domain.id})
        if (currentProject) {
          navigate('projects.project')
        } else {
          navigate('domains')
        }
      } else {
        notifications.show({
          title: 'Error',
          message: `Domain ${domain.domain} could not be deleted`,
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
        <h2>Domain Details</h2>
        <form onSubmit={form.onSubmit(save)}>
          <Stack>
            <TextInput
              label="Domain"
              key={form.key('domain')}
              value={form.values.domain}
              onChange={(event) => form.setFieldValue('domain', event.currentTarget.value)}
            />
            <Group>
              <Tooltip label='Delete Domain'>
                <Button leftSection={<IconTrash/>}
                        color="red"
                        onClick={() => confirmDeleteDomain(currentDomain)}>Delete</Button>
              </Tooltip>
              <Button type="submit" disabled={!form.isDirty()} loading={form.submitting}>Save</Button>
            </Group>
          </Stack>
        </form>
      </Grid.Col>
    </Grid>
  )
}