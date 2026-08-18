import { useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { ArrowDownToLine, ArrowUpFromLine, Gauge, RotateCw } from "lucide-react";
import {
  downloadPayload,
  MAX_DOWNLOAD_BYTES,
  MAX_UPLOAD_BYTES,
  MIN_DOWNLOAD_BYTES,
  MIN_UPLOAD_BYTES,
  nextPayloadSize,
  uploadPayload
} from "../../api/speedTestApi";
import { formatBytes } from "../../utils/adminFormat";
import { Button } from "../Button";
import { SettingsSection } from "./SettingsSection";

const DOWNLOAD_ROUNDS = 3;
const UPLOAD_ROUNDS = 3;
const INITIAL_DOWNLOAD_BYTES = 4 * 1024 * 1024;
const INITIAL_UPLOAD_BYTES = 2 * 1024 * 1024;

type Phase =
  | "idle"
  | "preparing"
  | "downloading"
  | "uploading"
  | "finalizing"
  | "completed"
  | "error";

type Direction = "download" | "upload";

type RoundResult = {
  direction: Direction;
  round: number;
  bytes: number;
  durationMs: number;
  mbps: number;
};

function formatMbps(value: number): string {
  return `${value.toFixed(1)} Mbps`;
}

function formatDuration(durationMs: number): string {
  return `${(durationMs / 1000).toFixed(1)} s`;
}

export function SpeedTestCard() {
  const { t } = useTranslation();
  const [phase, setPhase] = useState<Phase>("idle");
  const [progress, setProgress] = useState(0);
  const [error, setError] = useState("");
  const [results, setResults] = useState<RoundResult[]>([]);
  const [currentRound, setCurrentRound] = useState(0);
  const lastProgress = useRef(0);

  const running =
    phase === "preparing" ||
    phase === "downloading" ||
    phase === "uploading" ||
    phase === "finalizing";

  const { downloadMbps, uploadMbps, downloadBytes, uploadBytes } =
    useMemo(() => {
      const download = results.filter(result => result.direction === "download");
      const upload = results.filter(result => result.direction === "upload");
      const sum = (rounds: RoundResult[]) =>
        rounds.reduce(
          (total, round) => total + round.bytes,
          0
        );
      const duration = (rounds: RoundResult[]) =>
        rounds.reduce((total, round) => total + round.durationMs, 0);
      const downloadDurationMs = duration(download);
      const uploadDurationMs = duration(upload);
      const downloadBytesTotal = sum(download);
      const uploadBytesTotal = sum(upload);
      return {
        downloadMbps:
          downloadDurationMs > 0
            ? (downloadBytesTotal * 8) / downloadDurationMs / 1000
            : 0,
        uploadMbps:
          uploadDurationMs > 0
            ? (uploadBytesTotal * 8) / uploadDurationMs / 1000
            : 0,
        downloadBytes: downloadBytesTotal,
        uploadBytes: uploadBytesTotal
      };
    }, [results]);

  function updateProgress(received: number, total: number) {
    const percent = Math.min(100, Math.floor((received / total) * 100));
    if (percent !== lastProgress.current) {
      lastProgress.current = percent;
      setProgress(percent);
    }
  }

  async function runTest() {
    if (running) {
      return;
    }
    setPhase("preparing");
    setProgress(0);
    setError("");
    setResults([]);
    try {
      const nextRounds: RoundResult[] = [];
      let downloadSize = INITIAL_DOWNLOAD_BYTES;
      for (let round = 1; round <= DOWNLOAD_ROUNDS; round += 1) {
        setPhase("downloading");
        setCurrentRound(round);
        lastProgress.current = 0;
        setProgress(0);
        const result = await downloadPayload(downloadSize, received =>
          updateProgress(received, downloadSize)
        );
        nextRounds.push({ direction: "download", round, ...result });
        downloadSize = nextPayloadSize(
          result,
          downloadSize,
          MIN_DOWNLOAD_BYTES,
          MAX_DOWNLOAD_BYTES
        );
      }
      let uploadSize = INITIAL_UPLOAD_BYTES;
      for (let round = 1; round <= UPLOAD_ROUNDS; round += 1) {
        setPhase("uploading");
        setCurrentRound(round);
        lastProgress.current = 0;
        setProgress(0);
        const result = await uploadPayload(uploadSize, sent =>
          updateProgress(sent, uploadSize)
        );
        nextRounds.push({ direction: "upload", round, ...result });
        uploadSize = nextPayloadSize(
          result,
          uploadSize,
          MIN_UPLOAD_BYTES,
          MAX_UPLOAD_BYTES
        );
      }
      setPhase("finalizing");
      setResults(nextRounds);
      setProgress(100);
      setPhase("completed");
    } catch (runError) {
      setError(
        runError instanceof Error
          ? runError.message
          : t("setup.settings.speedTest.failed")
      );
      setPhase("error");
    }
  }

  const totalRounds = phase === "downloading" ? DOWNLOAD_ROUNDS : UPLOAD_ROUNDS;

  const statusLabel =
    phase === "preparing"
      ? t("setup.settings.speedTest.preparing")
      : phase === "downloading"
        ? t("setup.settings.speedTest.downloading", {
            round: currentRound,
            total: totalRounds
          })
        : phase === "uploading"
          ? t("setup.settings.speedTest.uploading", {
              round: currentRound,
              total: totalRounds
            })
          : phase === "finalizing"
            ? t("setup.settings.speedTest.finalizing")
            : phase === "completed"
              ? t("setup.settings.speedTest.completed")
              : phase === "error"
                ? t("setup.settings.speedTest.failed")
                : t("setup.settings.speedTest.empty");

  return (
    <SettingsSection
      className="settings-card--wide"
      title={t("setup.settings.speedTest.title")}
      description={t("setup.settings.speedTest.description")}
      icon={<Gauge size={20} />}
    >
      {error ? <p className="error-message">{error}</p> : null}

      <div className="speed-test-actions">
        <Button
          data-testid="run-speed-test"
          type="button"
          loading={phase === "preparing" || phase === "finalizing"}
          disabled={running}
          icon={
            phase === "completed" ? (
              <RotateCw size={17} aria-hidden="true" />
            ) : undefined
          }
          onClick={() => void runTest()}
        >
          {phase === "completed"
            ? t("setup.settings.speedTest.actionAgain")
            : t("setup.settings.speedTest.action")}
        </Button>
        {running || phase === "completed" ? (
          <span className="speed-test-status" role="status">
            {statusLabel}
          </span>
        ) : null}
      </div>

      {running ? (
        <div
          className="upload-progress-bar speed-test-progress"
          data-testid="speed-test-progress"
        >
          <span style={{ width: `${progress}%` }} />
        </div>
      ) : null}

      {phase === "completed" && results.length > 0 ? (
        <div className="speed-test-results" data-testid="speed-test-results">
          <div className="speed-test-summary">
            <SpeedTestSummaryItem
              icon={<ArrowDownToLine size={17} />}
              label={t("setup.settings.speedTest.download")}
              value={formatMbps(downloadMbps)}
              hint={t("setup.settings.speedTest.transferred", {
                amount: formatBytes(downloadBytes)
              })}
            />
            <SpeedTestSummaryItem
              icon={<ArrowUpFromLine size={17} />}
              label={t("setup.settings.speedTest.upload")}
              value={formatMbps(uploadMbps)}
              hint={t("setup.settings.speedTest.transferred", {
                amount: formatBytes(uploadBytes)
              })}
            />
          </div>
          <table className="settings-table speed-test-table">
            <thead>
              <tr>
                <th>{t("setup.settings.speedTest.round")}</th>
                <th>{t("setup.settings.speedTest.direction")}</th>
                <th>{t("setup.settings.speedTest.transferredAmount")}</th>
                <th>{t("setup.settings.speedTest.duration")}</th>
                <th>{t("setup.settings.speedTest.speed")}</th>
              </tr>
            </thead>
            <tbody>
              {results.map(result => (
                <tr key={`${result.direction}-${result.round}`}>
                  <td>{result.round}</td>
                  <td>
                    {result.direction === "download"
                      ? t("setup.settings.speedTest.download")
                      : t("setup.settings.speedTest.upload")}
                  </td>
                  <td>{formatBytes(result.bytes)}</td>
                  <td>{formatDuration(result.durationMs)}</td>
                  <td>{formatMbps(result.mbps)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : null}

      {!running && phase === "idle" ? (
        <p className="settings-hint">
          {t("setup.settings.speedTest.empty")}
        </p>
      ) : null}
    </SettingsSection>
  );
}

function SpeedTestSummaryItem({
  icon,
  label,
  value,
  hint
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
  hint: string;
}) {
  return (
    <div className="mcp-summary-item">
      {icon}
      <span>{label}</span>
      <strong>{value}</strong>
      <small>{hint}</small>
    </div>
  );
}