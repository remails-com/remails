import { useForm } from "@mantine/form";
import { Group, Select, Stack, Text } from "@mantine/core";
import { Domain } from "../../types.ts";
import { modals } from "@mantine/modals";
import { notifications } from "@mantine/notifications";
import { IconTrash } from "@tabler/icons-react";
import { useRemails } from "../../hooks/useRemails.ts";
import { useOrganizations } from "../../hooks/useOrganizations.ts";
import { Loader } from "../../Loader.tsx";
import { useDomains } from "../../hooks/useDomains.ts";
import { errorNotification } from "../../notify.tsx";
import { MaintainerButton } from "../RoleButtons.tsx";
import { useProjects } from "../../hooks/useProjects.ts";

interface FormValues {
  project_id: string | null;
}

export default function DomainSettings() {
  const { dispatch, navigate } = useRemails();

  const { currentOrganization } = useOrganizations();
  const { currentDomain } = useDomains();
  const { projects } = useProjects();

  const form = useForm<FormValues>({
    initialValues: {
      project_id: currentDomain?.project_id ?? null,
    },
  });

  if (!currentDomain || !currentOrganization) {
    return <Loader />;
  }

  const confirmDeleteDomain = (domain: Domain) => {
    modals.openConfirmModal({
      title: "Please confirm your action",
      children: (
        <Text>
          Are you sure you want to delete the domain <strong>{domain.domain}</strong>? This action cannot be undone.
        </Text>
      ),
      labels: { confirm: "Confirm", cancel: "Cancel" },
      onCancel: () => {},
      onConfirm: () => deleteDomain(domain),
    });
  };

  const deleteDomain = async (domain: Domain) => {
    const res = await fetch(`/api/organizations/${currentOrganization.id}/domains/${domain.id}`, {
      method: "DELETE",
    });
    if (res.status === 200) {
      notifications.show({
        title: "Domain deleted",
        message: `Domain ${domain.domain} deleted`,
        color: "green",
      });
      navigate("domains");
      dispatch({ type: "remove_domain", domainId: domain.id });
    } else {
      errorNotification(`Domain ${domain.domain} could not be deleted`);
      console.error(res);
    }
  };

  const save = async (values: FormValues) => {
    const res = await fetch(`/api/organizations/${currentOrganization.id}/domains/${currentDomain.id}`, {
      method: "PUT",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(values.project_id),
    });
    if (res.status !== 200) {
      errorNotification(`Project ${currentDomain.domain} could not be updated`);
      console.error(res);
      return;
    }
    const domain = await res.json();

    notifications.show({
      title: "Domain updated",
      message: "",
      color: "green",
    });
    dispatch({ type: "remove_domain", domainId: currentDomain.id });
    dispatch({ type: "add_domain", domain });
    form.resetDirty();
  };

  return (
    <>
      <h2>Domain Settings</h2>
      <form onSubmit={form.onSubmit(save)}>
        <Stack>
          <Select
            label="Usable by"
            placeholder="any project"
            data={projects.map((p) => ({ value: p.id, label: p.name }))}
            value={form.values.project_id}
            onChange={(project_id) => form.setFieldValue("project_id", project_id)}
            clearable
            searchable
            nothingFoundMessage="No project found..."
          />

          <Group mt="md">
            <MaintainerButton
              leftSection={<IconTrash />}
              variant="outline"
              tooltip="Delete project"
              onClick={() => confirmDeleteDomain(currentDomain)}
            >
              Delete
            </MaintainerButton>
            <MaintainerButton type="submit" disabled={!form.isDirty()} loading={form.submitting}>
              Save
            </MaintainerButton>
          </Group>
        </Stack>
      </form>
    </>
  );
}
