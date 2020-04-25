# DTOP

**dtop** is a tool designed to measure system utilization of applications and system performance. It calculates system load by a subtractive method. A background soaking task is executed on all system CPUs. If some new applications take up a certain amount  of the system computing power, the background program will lose those computing power accordingly. So the system utilization by the new applications can be evaluated from the lost.

**Build**
```
cargo build --release
```

**Usage**

**Calibrate the system**
dtop -c		// Calibrate the system with interval 1s.
dtop -c -i 5	// Calibrate the system with interval 5s.


**Measure system utilization of an application every 1s**

```
dtop -c		// Calibrate the system.
dtop		// Check the system utilization every 1s.
	... 	// Run an application on the system.
```

**Measure system utilization of an application every 5s**

```
dtop -c		// Calibrate the system.
dtop -i 5	// Check the system utilization every 5s.
	... 	// Run an application on the system.
```

**Measure system utilization of an application with step mode**

```
dtop -c		// Calibrate the system.
	... 	// Run an application on the system.
dtop -s		// Check the system utilization caused by the application.
```

**Measure a system performance**
```
dtop -c 	// Run this on a reference system, get the scores.txt.
	...	// Copy the scores.txt to the system being measured.
dtop		// Check the measuring system perforance with the reference system.
