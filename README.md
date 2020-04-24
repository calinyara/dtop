# DTOP

**dtop** is a tool designed to measure  the system load. It calculates system load by a subtractive method. A background soaking task is executed on all system CPUs. If some new workloads take up a certain amount  of the CPU computing power, the background program will lose those computing power accordingly. So the system load can be evaluated from the lost.



**Following is an example:**

```
dtop -c	 	// Calibrate the system CPU power.

	... 	// Run some workloads on the system.

dtop		//  Check the system load caused by the workload.
```
