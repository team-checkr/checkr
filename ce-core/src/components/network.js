const visPromise = import("https://esm.sh/vis-network/esnext");
const vis = await visPromise;

const id = "%id%";
const dot = "%dot%";

const data = vis.parseDOTNetwork(dot);

new vis.Network(document.getElementById(id), data, {
  interaction: { zoomView: false },
  nodes: {
    color: {
      background: "#666666",
      border: "#8080a0",
      highlight: "#80a0ff",
    },
    font: {
      color: "white",
    },
    borderWidth: 1,
    shape: "circle",
    size: 30,
  },
  edges: {
    color: "#D0D0FF",
    font: {
      color: "white",
      strokeColor: "#200020",
    },
  },
  autoResize: true,
});
