use std::collections::HashMap;

use crate::{
    AppState,
    handler::{
        api::{GraphQuery, GraphSpans},
        shell,
    },
};
use axum::{
    Router,
    extract::{Query, State},
    response::IntoResponse,
    routing::get,
};
use hypertext::{Raw, prelude::*};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_dashboard))
        .route("/dashboard/stats", get(get_stats))
}

pub async fn get_stats(
    State(state): State<AppState>,
    Query(params): Query<GraphQuery>,
) -> impl IntoResponse {
    // fetch your data the same way fetchData does
    let data = crate::handler::api::get_chart_data(params.span, &state)
        .await
        .unwrap_or_default();

    if data.is_empty() {
        return rsx! {
            <div class="stat-row" id="stat-row"
                hx-get=(format!("/dashboard/stats?span={}",params.span))
                hx-trigger="every 3s"
                hx-swap="outerHTML">
                // ... empty state
            </div>
        }
        .render();
    }

    let temps: Vec<f32> = data.iter().map(|d| d.temperature).collect();
    let current = temps[0];
    let min = temps.iter().cloned().fold(f32::INFINITY, f32::min);
    let max = temps.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let avg = temps.iter().sum::<f32>() / temps.len() as f32;

    rsx! {
        <div class="stat-row" id="stat-row"
            hx-get=(format!("/dashboard/stats?span={}",params.span))
            hx-trigger="every 3s"
            hx-swap="outerHTML">
            <div class="stat">
                <div class="stat-label">"Current"</div>
                <div class="stat-value">(format!("{:.1} °C", current))</div>
            </div>
            <div class="stat">
                <div class="stat-label">"Min"</div>
                <div class="stat-value">(format!("{:.1} °C", min))</div>
            </div>
            <div class="stat">
                <div class="stat-label">"Max"</div>
                <div class="stat-value">(format!("{:.1} °C", max))</div>
            </div>
            <div class="stat">
                <div class="stat-label">"Avg"</div>
                <div class="stat-value">(format!("{:.1} °C", avg))</div>
            </div>
        </div>
    }
    .render()
}

pub async fn get_dashboard() -> impl IntoResponse {
    shell(&rsx! {
        <div class="dashboard">
            <div class="header">
                <div class="header-left">
                    <div class="label">"sensor / ambient"</div>
                    <h1>"Temperature"</h1>
                </div>
                <div class="live-dot">"live"</div>
            </div>

            <div class="card">
                <div class="stat-row" id="stat-row"
                    hx-get="/dashboard/stats?span=1d"
                    hx-trigger="every 3s"
                    hx-swap="outerHTML">
                    <div class="stat">
                        <div class="stat-label">"Current"</div>
                        <div class="stat-value" id="stat-current">"—"</div>
                    </div>
                    <div class="stat">
                        <div class="stat-label">"Min"</div>
                        <div class="stat-value" id="stat-min">"—"</div>
                    </div>
                    <div class="stat">
                        <div class="stat-label">"Max"</div>
                        <div class="stat-value" id="stat-max">"—"</div>
                    </div>
                    <div class="stat">
                        <div class="stat-label">"Avg"</div>
                        <div class="stat-value" id="stat-avg">"—"</div>
                    </div>
                </div>

                <div class="span-buttons">
                    <button class="span-btn" data-span="5m">"5m"</button>
                    <button class="span-btn" data-span="30m">"30m"</button>
                    <button class="span-btn" data-span="1h">"1h"</button>
                    <button class="span-btn" data-span="6h">"6h"</button>
                    <button class="span-btn active" data-span="1d">"1d"</button>
                    <button class="span-btn" data-span="1w">"1w"</button>
                    <button class="span-btn" data-span="1m">"1m"</button>
                    <button class="span-btn" data-span="1y">"1y"</button>
                </div>

                <div id="chart"></div>
            </div>
        </div>

        <script> (Raw::dangerously_create(r##"
            const chartComponent = {
              chart: null,
              currentSpan: "1d",

              init() {
                this.chart = new ApexCharts(document.querySelector("#chart"), {
                  chart: {
                    type: "area",
                    height: 320,
                    background: "transparent",
                    toolbar: { show: false },
                    zoom: { enabled: false },
                    animations: { enabled: true, easing: "easeinout", speed: 400 },
                    fontFamily: "'DM Mono', monospace",
                  },
                  theme: { mode: "dark" },
                  series: [{ name: "°C", data: [] }],
                  xaxis: {
                    type: "datetime",
                    categories: [],
                    labels: {
                      style: { colors: "var(--text-color)", fontSize: "11px" },
                      datetimeUTC: false,
                    },
                    axisBorder: { show: false },
                    axisTicks: { show: false },
                  },
                  yaxis: {
                    labels: {
                      style: { colors: "var(--text-color)", fontSize: "11px" },
                      formatter: v => v != null ? v.toFixed(1) + "°" : "",
                    },
                  },
                  stroke: { curve: "smooth", width: 2, colors: ["#4fffb0"] },
                  fill: {
                    type: "gradient",
                    gradient: {
                      colorStops: [
                        { offset: 0,   color: "#4fffb0", opacity: 0.22 },
                        { offset: 100, color: "#4fffb0", opacity: 0    },
                      ],
                    },
                  },
                  grid: {
                    borderColor: "#1e2128",
                    strokeDashArray: 4,
                    xaxis: { lines: { show: false } },
                  },
                  tooltip: {
                    theme: "dark",
                    x: { format: "dd MMM HH:mm" },
                    y: { formatter: v => v != null ? v.toFixed(2) + " °C" : "" },
                    marker: { fillColors: ["#4fffb0"] },
                  },
                  markers: { size: 0 },
                  dataLabels: { enabled: false },
                });

                this.chart.render();
                this.fetchData(this.currentSpan);

                document.querySelectorAll(".span-btn").forEach(btn => {
                    btn.addEventListener("click", () => {
                        document.querySelectorAll(".span-btn").forEach(b => b.classList.remove("active"));
                        btn.classList.add("active");
                        this.currentSpan = btn.dataset.span;
                        this.fetchData(this.currentSpan);

                        // Sync the HTMX poll span and re-register
                        const row = document.getElementById("stat-row");
                        row.setAttribute("hx-get", `/dashboard/stats?span=${this.currentSpan}`);
                        htmx.process(row);
                    });
                });
                document.addEventListener("htmx:afterSwap", e => {
                    if (e.detail.target.id === "stat-row") {
                        this.fetchData(this.currentSpan);
                    }
                });
              },

              async fetchData(span) {
                try {
                  const resp = await fetch(`/api/v1/chart?span=${span}`);
                  const data = await resp.json();

                  const temps = data.map(d => d.temperature);
                  const cats  = data.map(d => new Date(d.timestamp).getTime());

                  this.chart.updateSeries([{ data: temps }], false);
                  this.chart.updateOptions({ xaxis: { categories: cats } }, false, true);
                } catch (err) {
                  console.error(err);
                }
              }
            };

            document.addEventListener("DOMContentLoaded", () => chartComponent.init());
        "##)) </script>
    })
    .render()
}
