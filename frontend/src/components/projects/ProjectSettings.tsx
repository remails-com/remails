import { useForm } from "@mantine/form";
import { Button, Group, Stack, Text, TextInput, Tooltip } from "@mantine/core";
import { Project } from "../../types.ts";
import { modals } from "@mantine/modals";
import { notifications } from "@mantine/notifications";
import { IconTrash, IconX } from "@tabler/icons-react";
import { useRemails } from "../../hooks/useRemails.ts";
import { useStreams } from "../../hooks/useStreams.ts";
import { useEffect } from "react";
import { useOrganizations } from "../../hooks/useOrganizations.ts";
import { useProjects } from "../../hooks/useProjects.ts";
import { Loader } from "../../Loader.tsx";

interface FormValues {
  name: string;
}

export default function ProjectSettings() {
  const { streams } = useStreams();
  const { dispatch, navigate } = useRemails();

  const { currentOrganization } = useOrganizations();
  const { currentProject } = useProjects();

  const canDelete = streams && streams.length === 0;

  const form = useForm<FormValues>({
    initialValues: {
      name: currentProject?.name || "",
    },
  });

  useEffect(() => {
    form.setValues({ name: currentProject?.name || "" });
    form.resetDirty();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentProject]);

  if (!currentProject || !currentOrganization) {
    return <Loader />;
  }

  const confirmDeleteProject = (project: Project) => {
    modals.openConfirmModal({
      title: "Please confirm your action",
      children: (
        <Text>
          Are you sure you want to delete project <strong>{project.name}</strong>? This action cannot be undone
        </Text>
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
      dispatch({ type: "remove_project", projectId: project.id });
      navigate("projects");
    } else {
      notifications.show({
        title: "Error",
        message: `Project ${project.name} could not be deleted`,
        color: "red",
        autoClose: 20000,
        icon: <IconX size={20} />,
      });
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
      notifications.show({
        title: "Error",
        message: `Project could not be updated`,
        color: "red",
        autoClose: 20000,
        icon: <IconX size={20} />,
      });
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
  };

  return (
    <>
      <h2>Project Settings</h2>
      <form onSubmit={form.onSubmit(save)}>
        <Stack>
          <TextInput
            label="Name"
            key={form.key("name")}
            value={form.values.name}
            onChange={(event) => form.setFieldValue("name", event.currentTarget.value)}
          />
          <Group>
            <Tooltip
              label={canDelete ? "Delete project" : "Cannot delete project, there are streams in it"}
              events={{ focus: false, hover: true, touch: true }}
            >
              <Button
                leftSection={<IconTrash />}
                variant="outline"
                disabled={!canDelete}
                onClick={() => confirmDeleteProject(currentProject)}
              >
                Delete
              </Button>
            </Tooltip>
            <Button type="submit" disabled={!form.isDirty()} loading={form.submitting}>
              Save
            </Button>
          </Group>
        </Stack>
      </form>
    </>
  );
}
