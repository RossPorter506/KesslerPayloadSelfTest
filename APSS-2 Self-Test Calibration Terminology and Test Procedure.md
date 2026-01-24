# Background

The APSS-2 payload board consists of many sensors and a few configurable power supplies. In isolation, each of these can be considered an open-loop sensor \- there is no feedback. But when a sensor and supply are taken together they form what is known as a closed-loop control system, where the readings from the sensor can be used to adjust the supply output to account for error. In this document I will describe the terminology used when discussing these topics.

With a simple open-loop sensor there are only two values that we care about, 

1. The 'actual', 'true', or 'ground truth' value \- the actual value in the real world, as measured by calibrated test equipment, e.g. multimeter, oscilloscope.   
2. The 'measured' value \- the value as measured by the sensor. 

We can define error as the difference between these two values. We can easily calibrate the measured value to bring it closer to the actual value, which minimises error.

In a closed-loop system where we can both set and read a value we suddenly have not two but three values:

1. The 'target' value \- the desired value the control system wants to set.   
2. The 'actual' value \- the value actually set by the circuit as measured by external calibrated equipment.   
3. The 'measured' value \- the value as measured by the sensing circuit.

This also means we now have three metrics we can choose to minimise, rather than just one. The target-actual error, actual-measured error, and target-measured error:

* The target-actual error, which I will call the 'setting' error defines the difference between the desired target value and the actual value produced by the circuit.   
* The actual-measured error, which I call the 'measurement' error, is the difference between the actual value and the value measured by the sensor.  
* The target-measured error, which I call the 'closed-loop' error, measures the difference between the desired value and the value measured by the circuit.

Naively calibrating against one metric can instead increase the other two metrics. For example, suppose the target value is 2V, the measured value is 2.5V and the actual is 3V. Calibrating the measured value down to match the target value decreases the closed-loop error but unintentionally increases the measurement error.

**Ideally calibration for such a setup would minimise setting error and measurement error**, which will in turn minimise the closed-loop error. Unfortunately calculating the measurement and setting error requires user intervention, and only closed-loop error can be calculated automatically.   
**Calibration designed to minimise closed-loop error directly can be deceptive and should be avoided.** For example, using the above example (target: 2V, measured: 2.5V, actual: 3V), if we tweak the measurement equation to produce 2V instead, the difference between the target and actual voltage remains unchanged.

To minimise the amount of manual tests required, we introduce the concept of an estimated value. The estimated value is derived from a model of the physical system stored in the controller and is used to estimate the actual value. For example, the current through a circuit may be difficult to measure, but with a sufficiently accurate measurement of the circuit resistance the current can be calculated fairly accurately, given an applied voltage. The efficacy of the estimated value can vary based on the complexity of the system, but for simple systems can be used as a reasonable approximation of the actual value.  
This allows the controller to *approximately* measure its own accuracy by measuring the closed-loop error.

When calibrating sensors, we try to remove any constant or linear error terms. These are also known as ‘offset’ and ‘gain’ errors respectively.

Offset errors must be removed first by ensuring that the zero-scale value for the measured and actual values match. Then, once there is no offset error, the gain error can be removed by multiplying the measured value such that the measured full-scale value matches the actual full-scale value.

**You must ensure the values do not overflow during any step of the calculation.** This may require you to clamp values so they don’t go below zero, for instance.

Because the MSP doesn’t have floating-point hardware, the offset and gain errors must be implemented in a manner that uses integers. For instance, if you need to multiply the output by 0.874 to correct the gain error, instead first multiply by 874 then divide by 1000, **paying careful attention to the order of operations to ensure precision is not lost early**.  
For instance meas \* (874 / 1000\) would mean meas \* 0 \= 0.

# Payload

The APSS-2 tether payload has the following sensors:

* Heater voltage,   
* Heater current,  
* Tether bias voltage,  
* Tether bias current,  
* Cathode offset voltage,   
* Cathode offset current,  
* Repeller voltage,  
* Aperture current,  
* Pinpuller current,

Additionally:  
LMS photodiode voltage 1, 2, 3

# 

# Calibration Procedures

### Heater Voltage

Configure the board for a [Standard Test](#standard-test).  
Clone the KesslerPCBSelfTest repository. Checkout the self-test branch. Program the board and check for any unexpected test failures. If all tests expected to pass are successful, continue the process.  
Update the code to call ManualPerformanceTests::test\_heater\_voltage() instead of the self-test. Program the board. Once programmed, power off the board and let it sit for 10 seconds.  
Power up the board and follow the instructions on the serial terminal. When prompted to measure the actual voltage, use the benchtop multimeter to measure the heater voltage using the test points marked HEATER+ and HEATER- (near the middle of the board). Round the results to the nearest enterable value and enter that into the terminal. Record the un-rounded value separately elsewhere.  
Repeat until the test completes. Copy the contents of the test from the terminal.  
Plot the results measured by the sensor against the measurements performed by the multimeter. If there is any obvious offset error or gain error, make a note of it.  
Calculate the terms needed to fix the offset and gain error. Apply these to the measured values (e.g. in excel) and see if they fix the offset and gain. If so, then apply these in the KesslerPayloadSelfTest repo.  
If you notice a difference between the set and actual values, note this down. (TODO)

### Tether Bias Voltage

Configure the board for a [Standard Test](#standard-test).  
Clone the KesslerPCBSelfTest repository. Checkout the self-test branch. Program the board and check for any unexpected test failures. If all tests expected to pass are successful, continue the process.  
Update the code to call ManualPerformanceTests::test\_tether\_bias\_voltage() instead of the self-test. Program the board. Once programmed, power off the board and let it sit for 10 seconds.  
Power up the board and follow the instructions on the serial terminal. When prompted to measure the actual voltage, use the benchtop multimeter to measure the tether bias voltage across pins 3 and 4 of PS2\_TBS (WARNING: HIGH VOLTAGE). Round the results to the nearest enterable value and enter that into the terminal. Record the un-rounded value separately elsewhere.  
Repeat until the test completes. Copy the contents of the test from the terminal.  
Plot the results measured by the sensor against the measurements performed by the multimeter. If there is any obvious offset error or gain error, make a note of it.  
Calculate the terms needed to fix the offset and gain error. Apply these to the measured values (e.g. in excel) and see if they fix the offset and gain. If so, then apply these in the KesslerPayloadSelfTest repo code.

### 

### Cathode Offset Voltage

Configure the board for a [Standard Test](#standard-test).  
Clone the KesslerPCBSelfTest repository. Checkout the self-test branch. Program the board and check for any unexpected test failures. If all tests expected to pass are successful, continue the process.  
Update the code to call ManualPerformanceTests::test\_cathode\_offset\_voltage() instead of the self-test. Program the board. Once programmed, power off the board and let it sit for 10 seconds.  
Power up the board and follow the instructions on the serial terminal. When prompted to measure the actual voltage, use the benchtop multimeter to measure the tether bias voltage across pins 3 and 4 of PS2\_COS (WARNING: HIGH VOLTAGE). Round the results to the nearest enterable value and enter that into the terminal. Record the un-rounded value separately elsewhere.  
Repeat until the test completes. Copy the contents of the test from the terminal.  
Plot the results measured by the sensor against the measurements performed by the multimeter. If there is any obvious offset error or gain error, make a note of it.  
Calculate the terms needed to fix the offset and gain error. Apply these to the measured values (e.g. in excel) and see if they fix the offset and gain. If so, then apply these in the KesslerPayloadSelfTest repo code.

### Repeller Voltage 

Configure the board for a [Standard Test](#standard-test).  
Clone the KesslerPCBSelfTest repository. Checkout the self-test branch. Program the board and check for any unexpected test failures. If all tests expected to pass are successful, continue the process.  
Update the code to call ManualPerformanceTests::test\_repeller\_voltage() instead of the self-test. Program the board. Once programmed, power off the board and let it sit for 10 seconds.  
Connect pin 3 of PS2\_TBS to the repeller plate.  
Power up the board and follow the instructions on the serial terminal. When prompted to measure the actual voltage, use the benchtop multimeter to measure the voltage from the repeller to the test point labelled HEATER- (near the middle of the board). Round the results to the nearest enterable value and enter that into the terminal. Record the un-rounded value separately elsewhere.  
Repeat until the test completes. Copy the contents of the test from the terminal.  
Plot the results measured by the sensor against the measurements performed by the multimeter. If there is any obvious offset error or gain error, make a note of it.  
Calculate the terms needed to fix the offset and gain error. Apply these to the measured values (e.g. in excel) and see if they fix the offset and gain. If so, then apply these in the KesslerPayloadSelfTest repo.

### Heater Current

Configure the board for an [Emitter Test](#emitter-test).  
Clone the KesslerPCBSelfTest repository. Checkout the self-test branch. Program the board and check for any unexpected test failures. If all tests expected to pass are successful, continue the process.  
Update the code to call ManualPerformanceTests::test\_heater\_current() instead of the self-test. Program the board. Once programmed, power off the board and let it sit for 10 seconds.  
Power up the board and follow the instructions on the serial terminal. When prompted to measure the actual voltage, use the benchtop multimeter to measure the voltage across the dummy heater. Using the known resistance of the dummy heater, convert the voltage across it to a current through it. Round the results to the nearest enterable value and enter that into the terminal. Record the un-rounded value separately elsewhere.  
Repeat until the test completes. Copy the contents of the test from the terminal.  
Plot the results measured by the sensor against the measurements performed by the multimeter. If there is any obvious offset error or gain error, make a note of it.  
Calculate the terms needed to fix the offset and gain error. Apply these to the measured values (e.g. in excel) and see if they fix the offset and gain. If so, then apply these in the KesslerPayloadSelfTest repo.

### Tether Bias Current

Configure the board for a [HVDC Current Test](#hvdc-current-test).  
Clone the KesslerPCBSelfTest repository. Checkout the self-test branch. Program the board and check for any unexpected test failures. If all tests expected to pass are successful, continue the process.  
Update the code to call ManualPerformanceTests::test\_tether\_bias\_current() instead of the self-test. Program the board. Once programmed, power off the board and let it sit for 10 seconds.  
Power up the board and follow the instructions on the serial terminal. When prompted to measure the actual current, use the benchtop multimeter to measure the voltage across the resistor across the tether bias supply (WARNING: HIGH VOLTAGE). Using the resistance of the resistor, convert the voltage across the resistor into a current through the resistor. Round the results to the nearest enterable value and enter that into the terminal. Record the un-rounded value separately elsewhere.  
Repeat until the test completes. Copy the contents of the test from the terminal.  
Plot the results measured by the sensor against the measurements performed by the multimeter. If there is any obvious offset error or gain error, make a note of it.  
Calculate the terms needed to fix the offset and gain error. Apply these to the measured values (e.g. in excel) and see if they fix the offset and gain. If so, then apply these in the KesslerPayloadSelfTest repo code.

### 

### Cathode Offset Current

Configure the board for a [HVDC Current Test](#hvdc-current-test).  
Clone the KesslerPCBSelfTest repository. Checkout the self-test branch. Program the board and check for any unexpected test failures. If all tests expected to pass are successful, continue the process.  
Update the code to call ManualPerformanceTests::test\_cathode\_offset\_current() instead of the self-test. Program the board. Once programmed, power off the board and let it sit for 10 seconds.  
Power up the board and follow the instructions on the serial terminal. When prompted to measure the actual current, use the benchtop multimeter to measure the voltage across the resistor across the cathode offset supply (WARNING: HIGH VOLTAGE). Using the known resistance of the resistor, convert the voltage across the resistor into a current through the resistor. Round the results to the nearest enterable value and enter that into the terminal. Record the un-rounded value separately elsewhere.  
Repeat until the test completes. Copy the contents of the test from the terminal.  
Plot the results measured by the sensor against the measurements performed by the multimeter. If there is any obvious offset error or gain error, make a note of it.  
Calculate the terms needed to fix the offset and gain error. Apply these to the measured values (e.g. in excel) and see if they fix the offset and gain. If so, then apply these in the KesslerPayloadSelfTest repo code.

### Aperture Current

Configure the board for an [Emitter Test](#emitter-test).  
Additionally prepare a power supply. Set the current limit to 11mA, and the voltage to 0V (for now). Connect the negative side to aperture. Connect the positive side to the exterior through a current-limiting 1K resistor (measure and record the resistance).  
Clone the KesslerPCBSelfTest repository. Checkout the self-test branch. Program the board and check for any unexpected test failures. If all tests expected to pass are successful, continue the process.  
Update the code to call ManualPerformanceTests::test\_aperture\_current() instead of the self-test. Program the board. Once programmed, power off the board and let it sit for 10 seconds.  
Power up the board and follow the instructions on the serial terminal. When a voltage is requested, apply that voltage to the power supply. Using the known resistance of the current limiting resistor (plus the 1 ohm resistance of the precision sense resistor on the board), convert the supply voltage into a current through the resistors. Note down this actual value plus the measured value provided by the terminal. Repeat this process until the test is complete. Copy the contents of the test from the terminal.  
Plot the results measured by the sensor against the measurements performed by the multimeter. If there is any obvious offset error or gain error, make a note of it.  
Calculate the terms needed to fix the offset and gain error. Apply these to the measured values (e.g. in excel) and see if they fix the offset and gain. If so, then apply these in the KesslerPayloadSelfTest repo.

### Pinpuller Current 

Configure the board for a [Pinpuller Test](#pinpuller-test).  
Clone the KesslerPCBSelfTest repository. Checkout the self-test branch. Program the board and check for any unexpected test failures. If all tests expected to pass are successful, continue the process.  
Update the code to call ManualPerformanceTests::test\_pinpuller\_current() instead of the self-test. Program the board. Once programmed, power off the board and let it sit for 10 seconds.  
Power up the board and follow the instructions on the serial terminal. When prompted to measure the actual current, use the benchtop multimeter to measure the voltage across the dummy pinpuller. Use the known resistance of the dummy pinpuller to convert the voltage across it to a current through it. Round the results to the nearest enterable value and enter that into the terminal. Record the un-rounded value separately elsewhere.  
Repeat until the test completes. Copy the contents of the test from the terminal.  
Plot the results measured by the sensor against the measurements performed by the multimeter. If there is any obvious offset error or gain error, make a note of it.  
Calculate the terms needed to fix the offset and gain error. Apply these to the measured values (e.g. in excel) and see if they fix the offset and gain. If so, then apply these in the KesslerPayloadSelfTest repo.

# Appendix

### Standard Test {#standard-test}

Prepare two power supplies:

* A power supply configured for 3.3V, 600mA.  
* A power supply configured for 5V, 1A.

Disable the output of both supplies. Connect the power supplies as so:

Enable both of the supplies at the same time.

### Emitter Test {#emitter-test}

Apply the Standard Test configuration. Before enabling the supplies connect J5a and J5b up to the dummy emitter board.

### HVDC Current Test {#hvdc-current-test}

Follow the Standard Test configuration. Before enabling the supplies, acquire the HVDC current test jig. Measure and note the individual resistance of the two resistors on the test jig. Mount the test jig on the board, ensuring the pogo pins are making contact with the target pads. You may wish to do a continuity test between each of these points on the board and the lead of the connected resistor on the test jig.

### 

### Pinpuller Test {#pinpuller-test}

Prepare three power supplies:

* A power supply configured for 3.7V, 3A (‘VBat’)  
* A power supply configured for 3.3V, 600mA. (‘3V3’)  
* A power supply configured for 5V, 1A. (‘5V’)

Disable the outputs of all the supplies. Connect the supplies as follows:

Enable the 5V and 3V3 supplies at the same time. Enable VBat afterwards.

