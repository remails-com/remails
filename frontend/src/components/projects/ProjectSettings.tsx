import { useForm } from "@mantine/form";
import { Group, List, Stack, Text, TextInput } from "@mantine/core";
import { Project } from "../../types.ts";
import { modals } from "@mantine/modals";
import { notifications } from "@mantine/notifications";
import { IconTrash } from "@tabler/icons-react";
import { useRemails } from "../../hooks/useRemails.ts";
import { useOrganizations } from "../../hooks/useOrganizations.ts";
import { useProjects } from "../../hooks/useProjects.ts";
import { Loader } from "../../Loader.tsx";
import { useDomains } from "../../hooks/useDomains.ts";
import { errorNotification } from "../../notify.tsx";
import { MaintainerButton } from "../RoleButtons.tsx";

interface FormValues {
  name: string;
}

export default function ProjectSettings() {
  const { dispatch, navigate } = useRemails();

  const { currentOrganization } = useOrganizations();
  const { currentProject } = useProjects();
  const { domains } = useDomains();

  const form = useForm<FormValues>({
    initialValues: {
      name: currentProject?.name || "",
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

  if (!currentProject || !currentOrganization) {
    return <Loader />;
  }

  const confirmDeleteProject = (project: Project) => {
    const project_domains = domains.filter((domain) => domain.project_id == project.id);

    modals.openConfirmModal({
      title: "Please confirm your action",
      size: "lg",
      children: (
        <>
          <Text>
            Are you sure you want to delete the <strong>{project.name}</strong> project?
          </Text>
          {project_domains.length > 0 && (
            <>
              <Text mt="sm">This will also delete the following domains linked to this project:</Text>
              <List>
                {project_domains.map((domain) => (
                  <List.Item key={domain.id}>
                    <Text fw="bold">{domain.domain}</Text>
                  </List.Item>
                ))}
              </List>
            </>
          )}
          <Text mt="sm">This action cannot be undone.</Text>
        </>
      ),
      labels: { confirm: "Confirm", cancel: "Cancel" },
      onCancel: () => {},
      onConfirm: () => deleteProject(project),
    });
  };

  const deleteProject = async (project: Project) => {
    const res = await fetch(`/api/organizations/${currentOrganization.id}/projects/${project.id}`, {
      method: "DELETE",
    });
    if (res.status === 200) {
      notifications.show({
        title: "Project deleted",
        message: `Project ${project.name} deleted`,
        color: "green",
      });
      navigate("projects");
      dispatch({ type: "remove_project", projectId: project.id });
    } else {
      errorNotification(`Project ${project.name} could not be deleted`);
      console.error(res);
    }
  };

  const save = async (values: FormValues) => {
    const res = await fetch(`/api/organizations/${currentOrganization.id}/projects/${currentProject.id}`, {
      method: "PUT",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(values),
    });
    if (res.status !== 200) {
      errorNotification(`Project ${currentProject.name} could not be updated`);
      console.error(res);
      return;
    }
    const project = await res.json();

    notifications.show({
      title: "Project updated",
      message: "",
      color: "green",
    });
    dispatch({ type: "remove_project", projectId: currentProject.id });
    dispatch({ type: "add_project", project });
    form.resetDirty();
  };

  return (
    <>
      <h2>Project Settings</h2>
      <form onSubmit={form.onSubmit(save)}>
        <Stack>
          <TextInput
            label="Name"
            error={form.errors.name}
            key={form.key("name")}
            value={form.values.name}
            onChange={(event) => form.setFieldValue("name", event.currentTarget.value)}
          />

          <Group mt="md">
            <MaintainerButton
              leftSection={<IconTrash />}
              variant="outline"
              tooltip="Delete project"
              onClick={() => confirmDeleteProject(currentProject)}
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
