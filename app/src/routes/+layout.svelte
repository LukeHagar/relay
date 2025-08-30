<script>
  import "../app.css";
  export let data;
  $: session = data.session;
</script>

<slot />

