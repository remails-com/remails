import { Button, Group, Stack, Text, TextInput, Tooltip } from "@mantine/core";
import { useForm } from "@mantine/form";
import { IconTrash, IconX } from "@tabler/icons-react";
import { useEffect } from "react";
import { useMessages } from "../../hooks/useMessages.ts";
import { Organization, Project } from "../../types.ts";
import { modals } from "@mantine/modals";
import { notifications } from "@mantine/notifications";
import { useRemails } from "../../hooks/useRemails.ts";

interface FormValues {
  name: string;
}

interface StreamSettingsProps {
  currentStream: Project;
  currentOrganization: Organization;
  currentProject: Project;
}

export default function StreamSettings({ currentStream, currentOrganization, currentProject }: StreamSettingsProps) {
  const { messages } = useMessages();
  const { dispatch, navigate } = useRemails();

  const canDelete = messages && messages.length === 0;

  const form = useForm<FormValues>({
    initialValues: {
      name: currentStream?.name || "",
    },
  });

  useEffect(() => {
    form.setValues({ name: currentStream?.name || "" });
    form.resetDirty();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentStream]);

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
      dispatch({ type: "remove_stream", streamId: stream.id });
      navigate("projects.project.streams");
    } else {
      notifications.show({
        title: "Error",
        message: `Stream ${stream.name} could not be deleted`,
        color: "red",
        autoClose: 20000,
        icon: <IconX size={20} />,
      });
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
      notifications.show({
        title: "Error",
        message: `Streams could not be updated`,
        color: "red",
        autoClose: 20000,
        icon: <IconX size={20} />,
      });
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
  };

  return (
    <>
      <h2>Stream Settings</h2>
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
              label={canDelete ? "Delete Stream" : "Cannot delete Stream, there are Messages in it"}
              events={{ focus: false, hover: true, touch: true }}
            >
              <Button
                leftSection={<IconTrash />}
                variant="outline"
                disabled={!canDelete}
                onClick={() => confirmDeleteStream(currentStream)}
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
