import type { GeneratedReport } from "@/hooks/useInfraReport";

export async function exportToPDF(report: GeneratedReport) {
  if (!report?.content) return;

  const { jsPDF } = await import("jspdf");
  const doc = new jsPDF({ orientation: "portrait", unit: "mm", format: "a4" });

  const pageWidth = doc.internal.pageSize.getWidth();
  const pageHeight = doc.internal.pageSize.getHeight();
  const marginLeft = 25;
  const marginRight = 25;
  const marginTop = 35;
  const marginBottom = 25;
  const contentWidth = pageWidth - marginLeft - marginRight;
  let y = marginTop;
  let pageNum = 1;

  const colors = {
    orange: [232, 93, 4] as [number, number, number],
    red: [208, 0, 0] as [number, number, number],
    yellow: [250, 163, 7] as [number, number, number],
    textDark: [45, 52, 54] as [number, number, number],
    textBody: [74, 74, 74] as [number, number, number],
    textMuted: [108, 117, 125] as [number, number, number],
    border: [222, 226, 230] as [number, number, number],
    bgLight: [248, 249, 250] as [number, number, number],
  };

  const drawDecorations = () => {
    doc.setFillColor(...colors.orange);
    doc.rect(0, 0, 5, pageHeight, "F");
    doc.setFillColor(...colors.yellow);
    doc.circle(pageWidth - 25, 25, 15, "F");
    doc.setFillColor(...colors.orange);
    doc.circle(pageWidth - 45, 40, 8, "F");

    const lineX = pageWidth - 15;
    doc.setLineWidth(2);
    doc.setDrawColor(...colors.red);
    doc.line(lineX, 60, lineX + 12, 60);
    doc.setDrawColor(...colors.orange);
    doc.line(lineX + 2, 66, lineX + 12, 66);
    doc.setDrawColor(...colors.yellow);
    doc.line(lineX + 4, 72, lineX + 12, 72);
    doc.setDrawColor(...colors.red);
    doc.line(lineX + 6, 78, lineX + 12, 78);

    const dotStartY = pageHeight - 60;
    const dotColors = [colors.orange, colors.yellow, colors.red];
    for (let row = 0; row < 3; row++) {
      for (let col = 0; col < 3; col++) {
        doc.setFillColor(...dotColors[(row + col) % 3]);
        doc.circle(pageWidth - 30 + col * 8, dotStartY + row * 8, 2, "F");
      }
    }
  };

  const setupPage = () => {
    drawDecorations();
    doc.setFontSize(9);
    doc.setTextColor(...colors.textMuted);
    doc.text(`Página ${pageNum}`, pageWidth - marginRight, pageHeight - 12, { align: "right" });
    doc.setFontSize(8);
    doc.text("Codazzy Infrastructure Monitoring", marginLeft, pageHeight - 12);
  };

  const checkPageBreak = (space: number) => {
    if (y + space > pageHeight - marginBottom) {
      doc.addPage();
      pageNum++;
      y = marginTop;
      setupPage();
    }
  };

  const addText = (
    text: string,
    fontSize: number,
    color: [number, number, number],
    opts: { bold?: boolean; indent?: number; lineHeight?: number } = {}
  ) => {
    const { bold = false, indent = 0, lineHeight = 1.5 } = opts;
    doc.setFontSize(fontSize);
    doc.setTextColor(...color);
    doc.setFont("helvetica", bold ? "bold" : "normal");

    const lines = doc.splitTextToSize(text, contentWidth - indent - 15);
    const actualHeight = fontSize * 0.4 * lineHeight;

    for (const line of lines) {
      checkPageBreak(actualHeight + 2);
      doc.text(line, marginLeft + indent, y);
      y += actualHeight;
    }
  };

  setupPage();

  doc.setFillColor(...colors.orange);
  doc.roundedRect(marginLeft, y - 8, 10, 10, 2, 2, "F");
  doc.setFontSize(14);
  doc.setTextColor(255, 255, 255);
  doc.setFont("helvetica", "bold");
  doc.text("C", marginLeft + 3.5, y - 1);
  doc.setFontSize(14);
  doc.setTextColor(...colors.textDark);
  doc.text("Codazzy", marginLeft + 14, y);
  y += 12;

  doc.setFontSize(22);
  doc.setTextColor(...colors.orange);
  doc.setFont("helvetica", "bold");
  const titleLines = doc.splitTextToSize(report.title, contentWidth - 20);
  for (const line of titleLines) {
    doc.text(line, marginLeft, y);
    y += 9;
  }
  y += 2;

  const dateStr = new Date(report.generatedAt).toLocaleDateString("es-ES");
  doc.setFontSize(11);
  doc.setTextColor(...colors.textMuted);
  doc.setFont("helvetica", "normal");
  doc.text(`Fecha: ${dateStr}`, marginLeft, y);
  y += 10;

  doc.setFillColor(...colors.bgLight);
  doc.roundedRect(marginLeft, y - 2, contentWidth - 15, 32, 2, 2, "F");
  doc.setDrawColor(...colors.orange);
  doc.setLineWidth(1.5);
  doc.line(marginLeft, y - 2, marginLeft, y + 30);
  y += 5;

  const infoItems = [
    ["Tipo de Informe:", report.type],
    ["Generado:", `${dateStr} - ${new Date(report.generatedAt).toLocaleTimeString("es-ES")}`],
    ["Servidores:", `${report.servers.length}`],
    ["Sistema:", "Codazzy Infrastructure Monitoring"],
  ];

  doc.setFontSize(10);
  for (const [label, value] of infoItems) {
    doc.setTextColor(...colors.textMuted);
    doc.text(label, marginLeft + 5, y);
    doc.setTextColor(...colors.textDark);
    doc.setFont("helvetica", "bold");
    doc.text(value, marginLeft + 45, y);
    doc.setFont("helvetica", "normal");
    y += 6;
  }
  y += 10;

  const lines = report.content.split("\n");

  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed) {
      y += 4;
      continue;
    }

    if (trimmed.startsWith("# ")) {
      y += 10;
      checkPageBreak(15);
      addText(trimmed.replace("# ", ""), 14, colors.textDark, { bold: true });
      doc.setDrawColor(...colors.orange);
      doc.setLineWidth(1);
      doc.line(marginLeft, y + 1, marginLeft + 50, y + 1);
      y += 6;
    } else if (trimmed.startsWith("## ")) {
      y += 8;
      addText(trimmed.replace("## ", ""), 12, colors.textDark, { bold: true });
      y += 3;
    } else if (trimmed.startsWith("### ")) {
      y += 5;
      addText(trimmed.replace("### ", ""), 11, colors.textBody, { bold: true });
      y += 2;
    } else if (trimmed.match(/^\d+\)\s/)) {
      y += 8;
      checkPageBreak(12);
      doc.setDrawColor(...colors.orange);
      doc.setLineWidth(2);
      doc.line(marginLeft, y - 3, marginLeft + 4, y - 3);
      addText(trimmed, 12, colors.textDark, { bold: true });
      y += 3;
    } else if (trimmed.startsWith("- ") || trimmed.startsWith("• ")) {
      checkPageBreak(10);
      doc.setFontSize(10);
      doc.setTextColor(...colors.orange);
      doc.text("—", marginLeft, y);
      addText(trimmed.replace(/^[-•]\s*/, ""), 10, colors.textBody, { indent: 8, lineHeight: 1.4 });
    } else if (trimmed.match(/^\s+[-•]/)) {
      checkPageBreak(8);
      doc.setFontSize(9);
      doc.setTextColor(...colors.textMuted);
      doc.text("–", marginLeft + 10, y);
      addText(trimmed.replace(/^\s*[-•]\s*/, ""), 9, colors.textMuted, { indent: 16, lineHeight: 1.3 });
    } else if (trimmed === "---" || trimmed === "***") {
      y += 5;
      doc.setDrawColor(...colors.border);
      doc.setLineWidth(0.5);
      doc.line(marginLeft, y, pageWidth - marginRight - 20, y);
      y += 7;
    } else if (trimmed.includes("**")) {
      addText(trimmed.replace(/\*\*/g, ""), 10, colors.textDark, { bold: true });
    } else {
      addText(trimmed, 10, colors.textBody, { lineHeight: 1.5 });
    }
  }

  y = pageHeight - 30;
  checkPageBreak(20);
  doc.setFillColor(255, 243, 205);
  doc.setDrawColor(255, 193, 7);
  doc.setLineWidth(0.5);
  doc.roundedRect(marginLeft, y, contentWidth - 15, 12, 2, 2, "FD");
  doc.setFontSize(8);
  doc.setTextColor(133, 100, 4);
  doc.setFont("helvetica", "bold");
  doc.text("Aviso:", marginLeft + 4, y + 7);
  doc.setFont("helvetica", "normal");
  doc.text("Este documento contiene información de infraestructura. Distribución restringida.", marginLeft + 18, y + 7);

  const filename = report.title?.replace(/\s+/g, "_").replace(/[^a-zA-Z0-9_-]/g, "") || `report_${Date.now()}`;
  doc.save(`${filename}.pdf`);
}

export function exportToMarkdown(report: GeneratedReport) {
  const filename = report.title.replace(/\s+/g, "_");
  const blob = new Blob([report.content], { type: "text/markdown" });
  downloadBlob(blob, `${filename}.md`);
}

export function exportDebugPrompt(report: GeneratedReport) {
  if (!report.debugPrompt) return;

  const content = `=== DEBUG PROMPT - ${report.title} ===
Generado: ${report.debugPrompt.timestamp}
Modelo: ${report.debugPrompt.model}

================================================================================
SYSTEM PROMPT
================================================================================
${report.debugPrompt.systemPrompt}

================================================================================
USER PROMPT
================================================================================
${report.debugPrompt.userPrompt}

================================================================================
METADATA
================================================================================
- Tipo de informe: ${report.type}
- Servidores: ${report.servers.join(", ")}
- Anomalías: ${report.anomaliesCount}
- Predicciones: ${report.predictionsCount}
`;

  const blob = new Blob([content], { type: "text/plain" });
  downloadBlob(blob, `debug_prompt_${report.id}.txt`);
}

function downloadBlob(blob: Blob, filename: string) {
  const url = window.URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  window.URL.revokeObjectURL(url);
  document.body.removeChild(a);
}

