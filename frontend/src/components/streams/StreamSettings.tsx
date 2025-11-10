import { Group, Stack, Text, TextInput } from "@mantine/core";
import { useForm } from "@mantine/form";
import { IconTrash } from "@tabler/icons-react";
import { useMessages } from "../../hooks/useMessages.ts";
import { Project } from "../../types.ts";
import { modals } from "@mantine/modals";
import { notifications } from "@mantine/notifications";
import { useRemails } from "../../hooks/useRemails.ts";
import { useOrganizations } from "../../hooks/useOrganizations.ts";
import { useProjects } from "../../hooks/useProjects.ts";
import { useStreams } from "../../hooks/useStreams.ts";
import { Loader } from "../../Loader.tsx";
import { errorNotification } from "../../notify.tsx";
import { MaintainerButton } from "../RoleButtons.tsx";

interface FormValues {
  name: string;
}

export default function StreamSettings() {
  const { messages } = useMessages();
  const { dispatch, navigate } = useRemails();
  const { currentOrganization } = useOrganizations();
  const { currentStream } = useStreams();
  const { currentProject } = useProjects();

  const canDelete = messages && messages.length === 0;

  const form = useForm<FormValues>({
    initialValues: {
      name: currentStream?.name ?? "",
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

  if (!currentStream || !currentOrganization || !currentProject) {
    return <Loader />;
  }

  const confirmDeleteStream = (project: Project) => {
    modals.openConfirmModal({
      title: "Please confirm your action",
      children: (
        <Text>
          Are you sure you want to delete Stream <strong>{project.name}</strong>? This action cannot be undone
        </Text>
      ),
      labels: { confirm: "Confirm", cancel: "Cancel" },
      onCancel: () => {},
      onConfirm: () => deleteStream(project),
    });
  };

  const deleteStream = async (stream: Project) => {
    const res = await fetch(
      `/api/organizations/${currentOrganization.id}/projects/${currentProject.id}/streams/${stream.id}`,
      {
        method: "DELETE",
      }
    );
    if (res.status === 200) {
      notifications.show({
        title: "Stream deleted",
        message: `Stream ${stream.name} deleted`,
        color: "green",
      });
      navigate("projects.project.streams");
      dispatch({ type: "remove_stream", streamId: stream.id });
    } else {
      errorNotification(`Stream ${stream.name} could not be deleted`);
      console.error(res);
    }
  };

  const save = async (values: FormValues) => {
    const res = await fetch(
      `/api/organizations/${currentOrganization.id}/projects/${currentProject.id}/streams/${currentStream.id}`,
      {
        method: "PUT",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify(values),
      }
    );
    if (res.status !== 200) {
      errorNotification("Stream could not be updated");
      console.error(res);
      return;
    }
    const stream = await res.json();

    notifications.show({
      title: "Stream updated",
      message: "",
      color: "green",
    });
    dispatch({ type: "remove_stream", streamId: currentStream.id });
    dispatch({ type: "add_stream", stream });
    form.resetDirty();
  };

  return (
    <>
      <h2>Stream Settings</h2>
      <form onSubmit={form.onSubmit(save)}>
        <Stack>
          <TextInput
            label="Name"
            key={form.key("name")}
            error={form.errors.name}
            value={form.values.name}
            onChange={(event) => form.setFieldValue("name", event.currentTarget.value)}
          />
          <Group>
            <MaintainerButton
              leftSection={<IconTrash />}
              variant="outline"
              disabled={!canDelete}
              tooltip={canDelete ? "Delete Stream" : "Cannot delete Stream, there are Messages in it"}
              onClick={() => confirmDeleteStream(currentStream)}
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
