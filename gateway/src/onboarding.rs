// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Gateway onboarding pages for trust.hypermesh.online.
//!
//! Provides self-contained HTML dashboards at three privilege levels:
//! - **Public**: Landing page with network stats and getting-started guide.
//! - **Private**: Authenticated peer view with topology, blocks, DNS, and catalog.
//! - **Admin**: Operator view with service health, config, and rate-limiter stats.
//!
//! All pages use inline CSS/JS with no external dependencies. The JavaScript
//! polls the gateway's `/api/status` endpoint for live data.

/// Landing page shown to unauthenticated visitors at trust.hypermesh.online.
///
/// Displays live network stats (refreshed every 10 s via `/api/status`) and a
/// step-by-step onboarding guide for joining the mesh.
pub const GATEWAY_PUBLIC_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>HyperMesh Network</title>
<style>
*{margin:0;padding:0;box-sizing:border-box}
body{background:#0a0a0a;color:#d0d0d0;font-family:'Courier New',monospace;line-height:1.6;padding:2rem}
a{color:#00ff88;text-decoration:none}
a:hover{text-decoration:underline}
h1{color:#00ff88;font-size:2rem;margin-bottom:.5rem}
h2{color:#00ff88;font-size:1.25rem;margin:2rem 0 1rem}
.container{max-width:800px;margin:0 auto}
.subtitle{color:#888;margin-bottom:2rem}
.stats{display:grid;grid-template-columns:repeat(auto-fit,minmax(200px,1fr));gap:1rem;margin-bottom:2rem}
.stat{background:#111;border:1px solid #222;border-radius:4px;padding:1rem}
.stat-label{color:#888;font-size:.75rem;text-transform:uppercase;letter-spacing:.05em}
.stat-value{color:#00ff88;font-size:1.5rem;font-weight:bold;margin-top:.25rem}
.stat-value.loading{color:#555}
.divider{border:none;border-top:1px solid #222;margin:2rem 0}
.step{margin-bottom:1.5rem}
.step-num{color:#00ff88;font-weight:bold;margin-bottom:.25rem}
pre{background:#111;border:1px solid #222;border-radius:4px;padding:1rem;overflow-x:auto;font-size:.85rem;margin:.5rem 0}
code{color:#e0e0e0}
.links{display:flex;gap:2rem;margin-top:2rem}
.footer{color:#555;font-size:.75rem;margin-top:3rem;text-align:center}
</style>
</head>
<body>
<div class="container">
<h1>HyperMesh Network</h1>
<p class="subtitle">Welcome to the HyperMesh decentralized mesh network.</p>

<h2>Network Status</h2>
<p style="color:#555;font-size:.75rem;margin-bottom:1rem">Live &mdash; refreshed every 10 s</p>
<div class="stats">
  <div class="stat">
    <div class="stat-label">Total Nodes</div>
    <div class="stat-value loading" id="node-count">&mdash;</div>
  </div>
  <div class="stat">
    <div class="stat-label">Network Blockchain Height</div>
    <div class="stat-value loading" id="chain-height">&mdash;</div>
  </div>
  <div class="stat">
    <div class="stat-label">Gateway Uptime</div>
    <div class="stat-value loading" id="uptime">&mdash;</div>
  </div>
</div>

<hr class="divider">

<h2>Get Started</h2>

<div class="step">
  <div class="step-num">1. Install</div>
  <pre><code>curl -LO https://github.com/hypermesh-online/core/releases/latest/download/hypermesh-linux-x86_64
chmod +x hypermesh-linux-x86_64 &amp;&amp; sudo mv hypermesh-linux-x86_64 /usr/local/bin/hypermesh</code></pre>
</div>

<div class="step">
  <div class="step-num">2. Join the mesh</div>
  <pre><code>hypermesh --privacy public --bootstrap "[2600:1900:4001:cf7::]:9292" connect</code></pre>
</div>

<div class="step">
  <div class="step-num">3. Register your domain</div>
  <pre><code>hypermesh domain register my-domain --privacy public</code></pre>
</div>

<div class="step">
  <div class="step-num">4. Deploy your dashboard</div>
  <pre><code>hypermesh dashboard init my-app &amp;&amp; hypermesh dashboard deploy ./my-app</code></pre>
</div>

<hr class="divider">

<div class="links">
  <a href="https://github.com/hypermesh-online/core/blob/main/papers/HYPERMESH.md">Documentation</a>
  <a href="https://github.com/hypermesh-online">Source</a>
</div>

<div class="footer">trust.hypermesh.online &mdash; HyperMesh Public Gateway</div>
</div>

<script>
(function(){
  function fmt(s){return s>=3600?(s/3600|0)+"h "+((s%3600)/60|0)+"m":s>=60?(s/60|0)+"m "+(s%60)+"s":s+"s"}
  function refresh(){
    fetch("/api/status").then(function(r){return r.json()}).then(function(d){
      var nc=document.getElementById("node-count");
      var ch=document.getElementById("chain-height");
      var up=document.getElementById("uptime");
      if(d.node_count!==undefined){nc.textContent=d.node_count;nc.classList.remove("loading")}
      if(d.chain_height!==undefined){ch.textContent=d.chain_height;ch.classList.remove("loading")}
      if(d.uptime_seconds!==undefined){up.textContent=fmt(d.uptime_seconds);up.classList.remove("loading")}
    }).catch(function(){})
  }
  refresh();
  setInterval(refresh,10000);
})();
</script>
</body>
</html>"##;

/// Dashboard shown to authenticated peers (PoS-validated identities).
///
/// Includes network topology, connected nodes with coordinates, a block
/// explorer, DNS registry browser, and catalog browser. All sections poll
/// `/api/private/*` endpoints for live data.
pub const GATEWAY_PRIVATE_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>HyperMesh &mdash; Network Dashboard</title>
<style>
*{margin:0;padding:0;box-sizing:border-box}
body{background:#0a0a0a;color:#d0d0d0;font-family:'Courier New',monospace;line-height:1.6;padding:2rem}
a{color:#00ff88;text-decoration:none}
a:hover{text-decoration:underline}
h1{color:#00ff88;font-size:1.75rem;margin-bottom:.25rem}
h2{color:#00ff88;font-size:1.1rem;margin:2rem 0 .75rem;border-bottom:1px solid #222;padding-bottom:.25rem}
.container{max-width:960px;margin:0 auto}
.subtitle{color:#888;margin-bottom:1.5rem;font-size:.85rem}
.tabs{display:flex;gap:.5rem;margin-bottom:1.5rem;flex-wrap:wrap}
.tab{background:#111;border:1px solid #222;border-radius:4px;padding:.4rem 1rem;cursor:pointer;color:#888;font-size:.85rem}
.tab.active{border-color:#00ff88;color:#00ff88}
.panel{display:none}
.panel.active{display:block}
table{width:100%;border-collapse:collapse;font-size:.8rem;margin-top:.5rem}
th{text-align:left;color:#00ff88;border-bottom:1px solid #222;padding:.4rem .5rem;font-weight:normal;text-transform:uppercase;font-size:.7rem;letter-spacing:.05em}
td{border-bottom:1px solid #111;padding:.4rem .5rem}
.mono{font-family:'Courier New',monospace}
.empty{color:#555;padding:1rem 0}
.badge{display:inline-block;background:#00ff8818;color:#00ff88;padding:0 .4rem;border-radius:2px;font-size:.7rem}
.stat-row{display:flex;gap:1rem;flex-wrap:wrap;margin-bottom:1rem}
.stat-card{background:#111;border:1px solid #222;border-radius:4px;padding:.75rem 1rem;flex:1;min-width:140px}
.stat-card .label{color:#888;font-size:.7rem;text-transform:uppercase}
.stat-card .value{color:#00ff88;font-size:1.25rem;font-weight:bold}
.footer{color:#555;font-size:.7rem;margin-top:3rem;text-align:center}
</style>
</head>
<body>
<div class="container">
<h1>HyperMesh Network Dashboard</h1>
<p class="subtitle">Authenticated peer view &mdash; data refreshes every 10 s</p>

<div class="stat-row">
  <div class="stat-card"><div class="label">Nodes</div><div class="value" id="p-nodes">&mdash;</div></div>
  <div class="stat-card"><div class="label">Chain Height</div><div class="value" id="p-height">&mdash;</div></div>
  <div class="stat-card"><div class="label">DNS Records</div><div class="value" id="p-dns">&mdash;</div></div>
  <div class="stat-card"><div class="label">Catalog Packages</div><div class="value" id="p-pkgs">&mdash;</div></div>
</div>

<div class="tabs">
  <div class="tab active" data-panel="topology">Topology</div>
  <div class="tab" data-panel="blocks">Block Explorer</div>
  <div class="tab" data-panel="dns">DNS Registry</div>
  <div class="tab" data-panel="catalog">Catalog</div>
</div>

<!-- Topology -->
<div class="panel active" id="panel-topology">
<h2>Network Topology</h2>
<table>
  <thead><tr><th>Node ID</th><th>Coordinates</th><th>Privacy</th><th>Status</th></tr></thead>
  <tbody id="topo-body"><tr><td colspan="4" class="empty">Loading...</td></tr></tbody>
</table>
</div>

<!-- Block Explorer -->
<div class="panel" id="panel-blocks">
<h2>Recent Blocks</h2>
<table>
  <thead><tr><th>Height</th><th>Hash</th><th>Transactions</th><th>Timestamp</th></tr></thead>
  <tbody id="blocks-body"><tr><td colspan="4" class="empty">Loading...</td></tr></tbody>
</table>
</div>

<!-- DNS Registry -->
<div class="panel" id="panel-dns">
<h2>DNS Registry</h2>
<table>
  <thead><tr><th>Domain</th><th>Owner</th><th>IPv6 Address</th><th>Registered</th></tr></thead>
  <tbody id="dns-body"><tr><td colspan="4" class="empty">Loading...</td></tr></tbody>
</table>
</div>

<!-- Catalog -->
<div class="panel" id="panel-catalog">
<h2>Catalog Packages</h2>
<table>
  <thead><tr><th>Name</th><th>Version</th><th>Type</th><th>Publisher</th></tr></thead>
  <tbody id="catalog-body"><tr><td colspan="4" class="empty">Loading...</td></tr></tbody>
</table>
</div>

<div class="footer">trust.hypermesh.online &mdash; Authenticated Peer Dashboard</div>
</div>

<script>
(function(){
  /* Tab switching */
  document.querySelectorAll(".tab").forEach(function(t){
    t.addEventListener("click",function(){
      document.querySelectorAll(".tab").forEach(function(x){x.classList.remove("active")});
      document.querySelectorAll(".panel").forEach(function(x){x.classList.remove("active")});
      t.classList.add("active");
      var p=document.getElementById("panel-"+t.getAttribute("data-panel"));
      if(p)p.classList.add("active");
    });
  });

  function esc(s){var d=document.createElement("div");d.textContent=s;return d.innerHTML}
  function truncHash(h){return h&&h.length>16?h.slice(0,8)+"..."+h.slice(-8):h||""}

  function renderRows(tbody,rows,cols){
    if(!rows||!rows.length){tbody.innerHTML='<tr><td colspan="'+cols+'" class="empty">No data available</td></tr>';return}
    tbody.innerHTML=rows.join("");
  }

  function refresh(){
    /* Summary stats */
    fetch("/api/status").then(function(r){return r.json()}).then(function(d){
      if(d.node_count!==undefined)document.getElementById("p-nodes").textContent=d.node_count;
      if(d.chain_height!==undefined)document.getElementById("p-height").textContent=d.chain_height;
      if(d.dns_count!==undefined)document.getElementById("p-dns").textContent=d.dns_count;
      if(d.catalog_count!==undefined)document.getElementById("p-pkgs").textContent=d.catalog_count;
    }).catch(function(){});

    /* Topology */
    fetch("/api/private/topology").then(function(r){return r.json()}).then(function(d){
      var rows=(d.nodes||[]).map(function(n){
        return "<tr><td class='mono'>"+esc(truncHash(n.id))+"</td>"
          +"<td class='mono'>("+n.x+","+n.y+","+n.z+")</td>"
          +"<td><span class='badge'>"+esc(n.privacy||"unknown")+"</span></td>"
          +"<td>"+esc(n.status||"connected")+"</td></tr>";
      });
      renderRows(document.getElementById("topo-body"),rows,4);
    }).catch(function(){});

    /* Blocks */
    fetch("/api/private/blocks").then(function(r){return r.json()}).then(function(d){
      var rows=(d.blocks||[]).map(function(b){
        return "<tr><td>"+b.height+"</td>"
          +"<td class='mono'>"+esc(truncHash(b.hash))+"</td>"
          +"<td>"+(b.tx_count||0)+"</td>"
          +"<td>"+esc(b.timestamp||"")+"</td></tr>";
      });
      renderRows(document.getElementById("blocks-body"),rows,4);
    }).catch(function(){});

    /* DNS */
    fetch("/api/private/dns").then(function(r){return r.json()}).then(function(d){
      var rows=(d.records||[]).map(function(r){
        return "<tr><td class='mono'>"+esc(r.domain)+"</td>"
          +"<td class='mono'>"+esc(truncHash(r.owner))+"</td>"
          +"<td class='mono'>"+esc(r.address||"")+"</td>"
          +"<td>"+esc(r.registered||"")+"</td></tr>";
      });
      renderRows(document.getElementById("dns-body"),rows,4);
    }).catch(function(){});

    /* Catalog */
    fetch("/api/private/catalog").then(function(r){return r.json()}).then(function(d){
      var rows=(d.packages||[]).map(function(p){
        return "<tr><td>"+esc(p.name)+"</td>"
          +"<td class='mono'>"+esc(p.version||"")+"</td>"
          +"<td><span class='badge'>"+esc(p.asset_type||"")+"</span></td>"
          +"<td class='mono'>"+esc(truncHash(p.publisher||""))+"</td></tr>";
      });
      renderRows(document.getElementById("catalog-body"),rows,4);
    }).catch(function(){});
  }

  refresh();
  setInterval(refresh,10000);
})();
</script>
</body>
</html>"##;

/// Admin dashboard shown only to the gateway operator.
///
/// Includes everything from the private dashboard plus service health
/// checks, gateway configuration, rate-limiter statistics, and connection
/// pool information. Polls `/api/admin/*` endpoints.
pub const GATEWAY_ADMIN_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>HyperMesh &mdash; Gateway Admin</title>
<style>
*{margin:0;padding:0;box-sizing:border-box}
body{background:#0a0a0a;color:#d0d0d0;font-family:'Courier New',monospace;line-height:1.6;padding:2rem}
a{color:#00ff88;text-decoration:none}
a:hover{text-decoration:underline}
h1{color:#00ff88;font-size:1.75rem;margin-bottom:.25rem}
h2{color:#00ff88;font-size:1.1rem;margin:2rem 0 .75rem;border-bottom:1px solid #222;padding-bottom:.25rem}
.container{max-width:1100px;margin:0 auto}
.subtitle{color:#888;margin-bottom:1.5rem;font-size:.85rem}
.tabs{display:flex;gap:.5rem;margin-bottom:1.5rem;flex-wrap:wrap}
.tab{background:#111;border:1px solid #222;border-radius:4px;padding:.4rem 1rem;cursor:pointer;color:#888;font-size:.85rem}
.tab.active{border-color:#00ff88;color:#00ff88}
.panel{display:none}
.panel.active{display:block}
table{width:100%;border-collapse:collapse;font-size:.8rem;margin-top:.5rem}
th{text-align:left;color:#00ff88;border-bottom:1px solid #222;padding:.4rem .5rem;font-weight:normal;text-transform:uppercase;font-size:.7rem;letter-spacing:.05em}
td{border-bottom:1px solid #111;padding:.4rem .5rem}
.mono{font-family:'Courier New',monospace}
.empty{color:#555;padding:1rem 0}
.badge{display:inline-block;padding:0 .4rem;border-radius:2px;font-size:.7rem}
.badge-ok{background:#00ff8818;color:#00ff88}
.badge-warn{background:#ffaa0018;color:#ffaa00}
.badge-err{background:#ff444418;color:#ff4444}
.stat-row{display:flex;gap:1rem;flex-wrap:wrap;margin-bottom:1rem}
.stat-card{background:#111;border:1px solid #222;border-radius:4px;padding:.75rem 1rem;flex:1;min-width:120px}
.stat-card .label{color:#888;font-size:.7rem;text-transform:uppercase}
.stat-card .value{color:#00ff88;font-size:1.25rem;font-weight:bold}
.config-grid{display:grid;grid-template-columns:minmax(180px,auto) 1fr;gap:0;font-size:.8rem}
.config-grid .ck{color:#888;padding:.3rem .5rem;border-bottom:1px solid #111}
.config-grid .cv{padding:.3rem .5rem;border-bottom:1px solid #111}
.footer{color:#555;font-size:.7rem;margin-top:3rem;text-align:center}
</style>
</head>
<body>
<div class="container">
<h1>Gateway Admin</h1>
<p class="subtitle">Operator dashboard &mdash; data refreshes every 10 s</p>

<div class="stat-row">
  <div class="stat-card"><div class="label">Nodes</div><div class="value" id="a-nodes">&mdash;</div></div>
  <div class="stat-card"><div class="label">Chain Height</div><div class="value" id="a-height">&mdash;</div></div>
  <div class="stat-card"><div class="label">Connections</div><div class="value" id="a-conns">&mdash;</div></div>
  <div class="stat-card"><div class="label">Uptime</div><div class="value" id="a-uptime">&mdash;</div></div>
  <div class="stat-card"><div class="label">Requests/s</div><div class="value" id="a-rps">&mdash;</div></div>
</div>

<div class="tabs">
  <div class="tab active" data-panel="health">Service Health</div>
  <div class="tab" data-panel="topology">Topology</div>
  <div class="tab" data-panel="blocks">Blocks</div>
  <div class="tab" data-panel="dns">DNS</div>
  <div class="tab" data-panel="catalog">Catalog</div>
  <div class="tab" data-panel="config">Config</div>
  <div class="tab" data-panel="ratelimit">Rate Limiter</div>
  <div class="tab" data-panel="pool">Conn Pool</div>
</div>

<!-- Service Health -->
<div class="panel active" id="panel-health">
<h2>Service Health</h2>
<table>
  <thead><tr><th>Service</th><th>Status</th><th>Uptime</th><th>Details</th></tr></thead>
  <tbody id="health-body"><tr><td colspan="4" class="empty">Loading...</td></tr></tbody>
</table>
</div>

<!-- Topology (same as private) -->
<div class="panel" id="panel-topology">
<h2>Network Topology</h2>
<table>
  <thead><tr><th>Node ID</th><th>Coordinates</th><th>Privacy</th><th>Status</th></tr></thead>
  <tbody id="topo-body"><tr><td colspan="4" class="empty">Loading...</td></tr></tbody>
</table>
</div>

<!-- Block Explorer -->
<div class="panel" id="panel-blocks">
<h2>Recent Blocks</h2>
<table>
  <thead><tr><th>Height</th><th>Hash</th><th>Transactions</th><th>Timestamp</th></tr></thead>
  <tbody id="blocks-body"><tr><td colspan="4" class="empty">Loading...</td></tr></tbody>
</table>
</div>

<!-- DNS -->
<div class="panel" id="panel-dns">
<h2>DNS Registry</h2>
<table>
  <thead><tr><th>Domain</th><th>Owner</th><th>IPv6 Address</th><th>Registered</th></tr></thead>
  <tbody id="dns-body"><tr><td colspan="4" class="empty">Loading...</td></tr></tbody>
</table>
</div>

<!-- Catalog -->
<div class="panel" id="panel-catalog">
<h2>Catalog Packages</h2>
<table>
  <thead><tr><th>Name</th><th>Version</th><th>Type</th><th>Publisher</th></tr></thead>
  <tbody id="catalog-body"><tr><td colspan="4" class="empty">Loading...</td></tr></tbody>
</table>
</div>

<!-- Gateway Config -->
<div class="panel" id="panel-config">
<h2>Gateway Configuration</h2>
<div class="config-grid" id="config-grid">
  <div class="ck">Loading...</div><div class="cv"></div>
</div>
</div>

<!-- Rate Limiter -->
<div class="panel" id="panel-ratelimit">
<h2>Rate Limiter Statistics</h2>
<div class="stat-row" id="rl-stats">
  <div class="stat-card"><div class="label">Allowed</div><div class="value" id="rl-allowed">&mdash;</div></div>
  <div class="stat-card"><div class="label">Rejected</div><div class="value" id="rl-rejected">&mdash;</div></div>
  <div class="stat-card"><div class="label">Active IPs</div><div class="value" id="rl-ips">&mdash;</div></div>
</div>
<h2>Top Rate-Limited IPs</h2>
<table>
  <thead><tr><th>IP Address</th><th>Requests</th><th>Rejected</th><th>Last Seen</th></tr></thead>
  <tbody id="rl-body"><tr><td colspan="4" class="empty">Loading...</td></tr></tbody>
</table>
</div>

<!-- Connection Pool -->
<div class="panel" id="panel-pool">
<h2>Connection Pool</h2>
<div class="stat-row">
  <div class="stat-card"><div class="label">Active</div><div class="value" id="pool-active">&mdash;</div></div>
  <div class="stat-card"><div class="label">Idle</div><div class="value" id="pool-idle">&mdash;</div></div>
  <div class="stat-card"><div class="label">Total Created</div><div class="value" id="pool-total">&mdash;</div></div>
</div>
<h2>Backends</h2>
<table>
  <thead><tr><th>Backend</th><th>Active</th><th>Idle</th><th>Health</th></tr></thead>
  <tbody id="pool-body"><tr><td colspan="4" class="empty">Loading...</td></tr></tbody>
</table>
</div>

<div class="footer">trust.hypermesh.online &mdash; Gateway Operator Console</div>
</div>

<script>
(function(){
  /* Tab switching */
  document.querySelectorAll(".tab").forEach(function(t){
    t.addEventListener("click",function(){
      document.querySelectorAll(".tab").forEach(function(x){x.classList.remove("active")});
      document.querySelectorAll(".panel").forEach(function(x){x.classList.remove("active")});
      t.classList.add("active");
      var p=document.getElementById("panel-"+t.getAttribute("data-panel"));
      if(p)p.classList.add("active");
    });
  });

  function esc(s){var d=document.createElement("div");d.textContent=String(s);return d.innerHTML}
  function truncHash(h){return h&&h.length>16?h.slice(0,8)+"..."+h.slice(-8):h||""}
  function fmt(s){return s>=3600?(s/3600|0)+"h "+((s%3600)/60|0)+"m":s>=60?(s/60|0)+"m "+(s%60)+"s":s+"s"}
  function badge(status){
    var cls=status==="healthy"||status==="up"?"badge-ok":status==="degraded"?"badge-warn":"badge-err";
    return "<span class='badge "+cls+"'>"+esc(status)+"</span>";
  }

  function renderRows(tbody,rows,cols){
    if(!rows||!rows.length){tbody.innerHTML='<tr><td colspan="'+cols+'" class="empty">No data available</td></tr>';return}
    tbody.innerHTML=rows.join("");
  }

  function refresh(){
    /* Summary */
    fetch("/api/status").then(function(r){return r.json()}).then(function(d){
      if(d.node_count!==undefined)document.getElementById("a-nodes").textContent=d.node_count;
      if(d.chain_height!==undefined)document.getElementById("a-height").textContent=d.chain_height;
      if(d.uptime_seconds!==undefined)document.getElementById("a-uptime").textContent=fmt(d.uptime_seconds);
    }).catch(function(){});

    /* Service Health */
    fetch("/api/admin/health").then(function(r){return r.json()}).then(function(d){
      if(d.connections!==undefined)document.getElementById("a-conns").textContent=d.connections;
      if(d.requests_per_second!==undefined)document.getElementById("a-rps").textContent=d.requests_per_second;
      var rows=(d.services||[]).map(function(s){
        return "<tr><td>"+esc(s.name)+"</td>"
          +"<td>"+badge(s.status||"unknown")+"</td>"
          +"<td>"+esc(s.uptime||"")+"</td>"
          +"<td>"+esc(s.details||"")+"</td></tr>";
      });
      renderRows(document.getElementById("health-body"),rows,4);
    }).catch(function(){});

    /* Topology */
    fetch("/api/private/topology").then(function(r){return r.json()}).then(function(d){
      var rows=(d.nodes||[]).map(function(n){
        return "<tr><td class='mono'>"+esc(truncHash(n.id))+"</td>"
          +"<td class='mono'>("+n.x+","+n.y+","+n.z+")</td>"
          +"<td><span class='badge badge-ok'>"+esc(n.privacy||"unknown")+"</span></td>"
          +"<td>"+esc(n.status||"connected")+"</td></tr>";
      });
      renderRows(document.getElementById("topo-body"),rows,4);
    }).catch(function(){});

    /* Blocks */
    fetch("/api/private/blocks").then(function(r){return r.json()}).then(function(d){
      var rows=(d.blocks||[]).map(function(b){
        return "<tr><td>"+b.height+"</td>"
          +"<td class='mono'>"+esc(truncHash(b.hash))+"</td>"
          +"<td>"+(b.tx_count||0)+"</td>"
          +"<td>"+esc(b.timestamp||"")+"</td></tr>";
      });
      renderRows(document.getElementById("blocks-body"),rows,4);
    }).catch(function(){});

    /* DNS */
    fetch("/api/private/dns").then(function(r){return r.json()}).then(function(d){
      var rows=(d.records||[]).map(function(r){
        return "<tr><td class='mono'>"+esc(r.domain)+"</td>"
          +"<td class='mono'>"+esc(truncHash(r.owner))+"</td>"
          +"<td class='mono'>"+esc(r.address||"")+"</td>"
          +"<td>"+esc(r.registered||"")+"</td></tr>";
      });
      renderRows(document.getElementById("dns-body"),rows,4);
    }).catch(function(){});

    /* Catalog */
    fetch("/api/private/catalog").then(function(r){return r.json()}).then(function(d){
      var rows=(d.packages||[]).map(function(p){
        return "<tr><td>"+esc(p.name)+"</td>"
          +"<td class='mono'>"+esc(p.version||"")+"</td>"
          +"<td><span class='badge badge-ok'>"+esc(p.asset_type||"")+"</span></td>"
          +"<td class='mono'>"+esc(truncHash(p.publisher||""))+"</td></tr>";
      });
      renderRows(document.getElementById("catalog-body"),rows,4);
    }).catch(function(){});

    /* Config */
    fetch("/api/admin/config").then(function(r){return r.json()}).then(function(d){
      var grid=document.getElementById("config-grid");
      var html="";
      var keys=Object.keys(d);
      if(!keys.length){grid.innerHTML='<div class="ck">No configuration data</div><div class="cv"></div>';return}
      keys.forEach(function(k){
        html+="<div class='ck'>"+esc(k)+"</div><div class='cv mono'>"+esc(String(d[k]))+"</div>";
      });
      grid.innerHTML=html;
    }).catch(function(){});

    /* Rate Limiter */
    fetch("/api/admin/rate-limiter").then(function(r){return r.json()}).then(function(d){
      if(d.allowed!==undefined)document.getElementById("rl-allowed").textContent=d.allowed;
      if(d.rejected!==undefined)document.getElementById("rl-rejected").textContent=d.rejected;
      if(d.active_ips!==undefined)document.getElementById("rl-ips").textContent=d.active_ips;
      var rows=(d.top_ips||[]).map(function(ip){
        return "<tr><td class='mono'>"+esc(ip.address)+"</td>"
          +"<td>"+ip.requests+"</td>"
          +"<td>"+ip.rejected+"</td>"
          +"<td>"+esc(ip.last_seen||"")+"</td></tr>";
      });
      renderRows(document.getElementById("rl-body"),rows,4);
    }).catch(function(){});

    /* Connection Pool */
    fetch("/api/admin/pool").then(function(r){return r.json()}).then(function(d){
      if(d.active!==undefined)document.getElementById("pool-active").textContent=d.active;
      if(d.idle!==undefined)document.getElementById("pool-idle").textContent=d.idle;
      if(d.total_created!==undefined)document.getElementById("pool-total").textContent=d.total_created;
      var rows=(d.backends||[]).map(function(b){
        return "<tr><td>"+esc(b.name)+"</td>"
          +"<td>"+b.active+"</td>"
          +"<td>"+b.idle+"</td>"
          +"<td>"+badge(b.health||"unknown")+"</td></tr>";
      });
      renderRows(document.getElementById("pool-body"),rows,4);
    }).catch(function(){});
  }

  refresh();
  setInterval(refresh,10000);
})();
</script>
</body>
</html>"##;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_html_is_valid_document() {
        assert!(GATEWAY_PUBLIC_HTML.starts_with("<!DOCTYPE html>"));
        assert!(GATEWAY_PUBLIC_HTML.contains("<title>HyperMesh Network</title>"));
        assert!(GATEWAY_PUBLIC_HTML.contains("/api/status"));
        assert!(GATEWAY_PUBLIC_HTML.contains("node-count"));
        assert!(GATEWAY_PUBLIC_HTML.contains("chain-height"));
        assert!(GATEWAY_PUBLIC_HTML.contains("uptime"));
        assert!(GATEWAY_PUBLIC_HTML.contains("Get Started"));
        assert!(GATEWAY_PUBLIC_HTML.contains("hypermesh-linux-x86_64"));
    }

    #[test]
    fn private_html_has_all_sections() {
        assert!(GATEWAY_PRIVATE_HTML.starts_with("<!DOCTYPE html>"));
        assert!(GATEWAY_PRIVATE_HTML.contains("panel-topology"));
        assert!(GATEWAY_PRIVATE_HTML.contains("panel-blocks"));
        assert!(GATEWAY_PRIVATE_HTML.contains("panel-dns"));
        assert!(GATEWAY_PRIVATE_HTML.contains("panel-catalog"));
        assert!(GATEWAY_PRIVATE_HTML.contains("/api/private/topology"));
        assert!(GATEWAY_PRIVATE_HTML.contains("/api/private/blocks"));
        assert!(GATEWAY_PRIVATE_HTML.contains("/api/private/dns"));
        assert!(GATEWAY_PRIVATE_HTML.contains("/api/private/catalog"));
    }

    #[test]
    fn admin_html_has_operator_sections() {
        assert!(GATEWAY_ADMIN_HTML.starts_with("<!DOCTYPE html>"));
        // Admin-only panels
        assert!(GATEWAY_ADMIN_HTML.contains("panel-health"));
        assert!(GATEWAY_ADMIN_HTML.contains("panel-config"));
        assert!(GATEWAY_ADMIN_HTML.contains("panel-ratelimit"));
        assert!(GATEWAY_ADMIN_HTML.contains("panel-pool"));
        // Admin-only endpoints
        assert!(GATEWAY_ADMIN_HTML.contains("/api/admin/health"));
        assert!(GATEWAY_ADMIN_HTML.contains("/api/admin/config"));
        assert!(GATEWAY_ADMIN_HTML.contains("/api/admin/rate-limiter"));
        assert!(GATEWAY_ADMIN_HTML.contains("/api/admin/pool"));
        // Also includes private-level data
        assert!(GATEWAY_ADMIN_HTML.contains("panel-topology"));
        assert!(GATEWAY_ADMIN_HTML.contains("panel-blocks"));
        assert!(GATEWAY_ADMIN_HTML.contains("panel-dns"));
        assert!(GATEWAY_ADMIN_HTML.contains("panel-catalog"));
    }

    #[test]
    fn all_pages_use_dark_theme() {
        for html in [GATEWAY_PUBLIC_HTML, GATEWAY_PRIVATE_HTML, GATEWAY_ADMIN_HTML] {
            assert!(html.contains("#0a0a0a"), "background color missing");
            assert!(html.contains("#00ff88"), "accent color missing");
            assert!(html.contains("monospace"), "monospace font missing");
        }
    }

    #[test]
    fn no_external_dependencies() {
        for html in [GATEWAY_PUBLIC_HTML, GATEWAY_PRIVATE_HTML, GATEWAY_ADMIN_HTML] {
            // No CDN links, no external scripts/stylesheets
            assert!(!html.contains("cdn."), "must not reference CDN");
            assert!(!html.contains("googleapis.com"), "must not reference external APIs");
            assert!(!html.contains("<link rel=\"stylesheet\" href=\"http"));
            assert!(!html.contains("<script src=\"http"));
        }
    }

    #[test]
    fn public_html_size_is_reasonable() {
        // Should be under 8 KB for a landing page
        assert!(
            GATEWAY_PUBLIC_HTML.len() < 8192,
            "public HTML is {} bytes, expected < 8192",
            GATEWAY_PUBLIC_HTML.len()
        );
    }
}
