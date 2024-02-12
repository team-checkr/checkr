<article class="prose prose-invert mx-auto">
  <h1 id="guide">Guide</h1>
  <p>
    This is a guide to the functions of the <em>Inspectify</em>
    user-interface!
  </p>
  <h2 id="programs">Programs</h2>
  <p>
    To generate new programs, click on the <code>Generate</code> button in the top left corner. This
    will randomly generate a valid GCL program. In the case you want to specify your own program, you
    can write in the box by clicking on the line you wish to change.
  </p>
  <p>
    To the left is a <code>Determinism</code> button which defines the determinism of the program, to
    change between non-determinism and determinism simply click on the button.
  </p>
  <p>
    To change the analysis for which you want to validate, choose from the analysis list by clicking
    on the environment button which defaults to <code>Interpreter</code>.
  </p>
  <h2 id="input">Input</h2>
  <p>The input is automatically generated randomly and cannot be changed.</p>
  <h2 id="output">Output</h2>
  <p>
    The output is shown at the bottom of the page. Given that your program produces an output in the
    correct format, a table containing the output of the analysis will be shown at the bottom left.
    In the case it does not produce an output in the expected format, either because the program
    crashed, an malformed output, or a rouge print statement, the
    <code>Debug output</code> will be showed instead.
  </p>
  <p>
    At the bottom right, you can choose between showing the
    <code>Reference output</code> with a table of the expected output of the analysis, the
    <code>Raw output</code>
    which shows the raw
    <code>stdout</code>, or <code>Debug output</code> which shows raw
    <code>stderr</code>. To switch between those, click on the title of the box, which is defaulted
    to
    <code>Reference output</code>.
  </p>
  <h3 id="validation">Validation</h3>
  <p>
    The line above the <code>Output</code> is representing the validation of the program, for which the
    color and symbol represent the following:
  </p>
  <ul>
    <!-- <li><p><Indicator state={IndicatorState.Working} /> Working</p></li>
	<li><p><Indicator state={IndicatorState.Correct} /> Correct</p></li>
	<li><p><Indicator state={IndicatorState.Mismatch} /> Mismatch</p></li>
	<li><p><Indicator state={IndicatorState.TimeOut} /> Time-out</p></li>
	<li><p><Indicator state={IndicatorState.Error} /> Error</p></li> -->
  </ul>
  <h2 id="graph">Graph</h2>
  <p>
    The default of the graph output will be that of your own implementation. To switch between your
    graph and the reference graph, use the <code>G</code> and <code>R</code> buttons to the right.
    <code>G</code> is the graph produced by your code and <code>R</code> is the graph from the
    reference implementation. The final button
    <code>S</code> is a split view, showing both the graph produced by your code and the the graph from
    the reference implementation.
  </p>
</article>
