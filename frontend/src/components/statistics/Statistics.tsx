import Quota from "./Quota.tsx";
import InfoAlert from "../InfoAlert.tsx";
import OrganizationHeader from "../organizations/OrganizationHeader.tsx";
import { Group, Stack } from "@mantine/core";
import TotalDelivered from "./TotalDelivered.tsx";
import SuccessPercentage from "./SuccessPercentage.tsx";
import PerMonthChart from "./PerMonthChart.tsx";
import DailyChart from "./DailyChart.tsx";

export default function Statistics() {
  return (
    <>
      <OrganizationHeader />
      <InfoAlert stateName="statistics">
        This section shows statistics about the statuses of emails sent by this organization, as well as your current
        usage and monthly sending quota.
      </InfoAlert>

      <Stack>
        <Group justify="space-evenly">
          <TotalDelivered />
          <SuccessPercentage />
          <Quota />
        </Group>

        <PerMonthChart />
        <DailyChart />
      </Stack>
    </>
  );
}
