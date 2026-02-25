import { Button, Group, Modal, Slider, Stack, TextInput, Title } from "@mantine/core";
import { useForm } from "@mantine/form";
import { useOrganizations } from "../../hooks/useOrganizations.ts";
import { useRemails } from "../../hooks/useRemails.ts";
import { notifications } from "@mantine/notifications";
import { errorNotification } from "../../notify.tsx";
import InfoTooltip from "../InfoTooltip.tsx";
import { MAX_RETENTION } from "./ProjectSettings.tsx";
import { useSubscription } from "../../hooks/useSubscription.ts";

interface FormValues {
  name: string;
  retention_period_days: number;
}

interface NewProjectProps {
  opened: boolean;
  close: () => void;
}

export function NewProject({ opened, close }: NewProjectProps) {
  const { currentOrganization } = useOrganizations();
  const { currentProduct } = useSubscription();
  const { navigate, dispatch } = useRemails();

  const form = useForm<FormValues>({
    initialValues: {
      name: "",
      retention_period_days: 1,
    },
    validate: {
      name: (value) => (value.length < 3 ? "Name must have at least 3 letters" : null),
    },
  });

  if (!currentOrganization) {
    return <></>;
  }

  const save = (values: FormValues) => {
    fetch(`/api/organizations/${currentOrganization.id}/projects`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(values),
    }).then((res) => {
      if (res.status === 201) {
        close();
        res.json().then((newProject) => {
          dispatch({ type: "add_project", project: newProject });
          navigate("projects.project.credentials", { proj_id: newProject.id });
          notifications.show({
            title: "Project created",
            message: `Project ${newProject.name} created`,
            color: "green",
          });
        });
      } else if (res.status === 409) {
        res.json().then((body) => {
          // differentiate between database conflict and project limit conflicts
          const description =
            body.description == "Conflict" ? "A project with this name already exists." : body.description;

          form.setFieldError("name", description);
        });
      } else {
        errorNotification(`Project ${values.name} could not be created`);
        console.error(res);
      }
    });
  };

  const max_retention = currentProduct ? MAX_RETENTION[currentProduct] : 1;

  return (
    <>
      <Modal
        opened={opened}
        onClose={close}
        title={
          <Title order={3} component="span">Create new project</Title>
        }
        size="lg"
        padding="xl"
      >
        <form onSubmit={form.onSubmit(save)}>
          <Stack gap="md">
            <TextInput
              data-autofocus
              label="Name"
              key={form.key("name")}
              value={form.values.name}
              placeholder="New project"
              error={form.errors.name}
              onChange={(event) => form.setFieldValue("name", event.currentTarget.value)}
            />

            <Stack gap="xs">
              <Group gap="xs" fz="sm" mb={0}>
                Email retention period (max. {max_retention} day){" "}
                <InfoTooltip
                  text="Sets how long emails in this project should remain available for inspection in Remails before deletion. The maximum retention period is based on your organization's Remails subscription."
                  size="xs"
                />
              </Group>
              <Slider
                px="xl"
                label={(value) => (value == 1 ? "1 day" : `${value} days`)}
                domain={[0, 30]}
                min={1}
                max={max_retention}
                value={form.values.retention_period_days}
                onChange={(value) => form.setFieldValue("retention_period_days", value)}
                marks={[
                  { value: 1, label: "1 day" },
                  { value: 7, label: "7 days" },
                  { value: 14, label: "14 days" },
                  { value: 30, label: "30 days" },
                ]}
              />
            </Stack>

            <Group justify="space-between" mt="xl">
              <Button onClick={close} variant="outline">
                Cancel
              </Button>
              <Button type="submit" loading={form.submitting}>
                Save
              </Button>
            </Group>
          </Stack>
        </form>
      </Modal>
    </>
  );
}
