import math

num_steps = 256
step = 1 / (num_steps)
print(step)
for i in range(num_steps):
    x = 1 + i * step
    y = math.log2(x)
    print(f'"{y:.18f}",')
